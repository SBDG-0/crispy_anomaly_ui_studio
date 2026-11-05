#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod document;
mod assets;
mod canvas_tools;
use canvas_tools::{Drag,AtlasCamera,OUTPUTS};
use assets::Index;
use document::Document;
use eframe::egui::{self, Color32, Pos2, Rect, Vec2, Stroke};
use std::{collections::{BTreeMap,HashMap,HashSet},path::{Path,PathBuf},sync::mpsc};

type Edits=BTreeMap<(usize,String),String>;
#[derive(Clone)]
struct Layer {root:PathBuf,enabled:bool}
type Decoded=(u64,PathBuf,Result<egui::ColorImage,String>);
struct App {
    root:String, layers:Vec<Layer>, files:Vec<PathBuf>, file:Option<PathBuf>, doc:Option<Document>,
    edits:Edits, undo:Vec<Edits>, redo:Vec<Edits>, selected:Option<usize>, filter:String,
    status:String, zoom:f32, ratio:usize, bounds:bool, preview:f32,
    index:Index, receiver:Option<mpsc::Receiver<Index>>, textures:HashMap<PathBuf,egui::TextureHandle>, failed:HashMap<PathBuf,String>,
    drag_start:Option<Drag>, resize:bool, show_xml:bool,
    texture_tx:mpsc::Sender<Decoded>, texture_rx:mpsc::Receiver<Decoded>, pending:HashSet<PathBuf>, hidden:HashSet<usize>,
    base_enabled:bool,file_filter:String,fit_requested:bool,atlas_height:f32,ui_font:f32,atlas_drag:bool,view_offset:Vec2,pan_pointer:Option<Pos2>,
    atlas_camera:AtlasCamera,atlas_key:Option<(PathBuf,Option<[f32;4]>)>,region_native:bool,region_pan:Vec2,region_pointer:Option<Pos2>,
    generation:u64, loaded_source:Option<PathBuf>, tree_expand:bool, tree_epoch:u64,
}
impl Default for App {
    fn default()->Self { let (texture_tx,texture_rx)=mpsc::channel(); Self{atlas_camera:AtlasCamera::default(),atlas_key:None,region_native:false,region_pan:Vec2::ZERO,region_pointer:None,view_offset:Vec2::ZERO,pan_pointer:None,atlas_drag:false,atlas_height:240.0,ui_font:14.0,base_enabled:true,file_filter:String::new(),fit_requested:false,generation:0,loaded_source:None,tree_expand:false,tree_epoch:0,texture_tx,texture_rx,pending:HashSet::new(),hidden:HashSet::new(),root:r"I:\_stalker.anomaly.multiplayer\Anomaly-1.5.3-Full\tools\_unpacked".into(),layers:Vec::new(),files:Vec::new(),file:None,doc:None,edits:Edits::new(),undo:Vec::new(),redo:Vec::new(),selected:None,filter:String::new(),status:"Choose a gamedata root, then click Scan project.".into(),zoom:1.0,ratio:0,bounds:true,preview:0.75,index:Index::default(),receiver:None,textures:HashMap::new(),failed:HashMap::new(),drag_start:None,resize:false,show_xml:false} }
}
impl App {
    fn roots(&self)->Vec<PathBuf>{std::iter::once(PathBuf::from(&self.root)).filter(|_|self.base_enabled).chain(self.layers.iter().filter(|l|l.enabled).map(|l|l.root.clone())).collect()}
    fn resolve(&self, relative:&Path)->Option<PathBuf>{assets::resolve(&self.roots(),relative)}
    fn scan(&mut self,ctx:&egui::Context){
        if !self.edits.is_empty(){self.status="Export or discard edits before changing source layers.".into();return}
        self.generation+=1;let roots=self.roots();let mut files=BTreeMap::new();
        for root in &roots {for e in walkdir::WalkDir::new(root.join("configs/ui")).sort_by_file_name().into_iter().filter_map(Result::ok){if e.file_type().is_file() && e.path().extension().is_some_and(|s|s.eq_ignore_ascii_case("xml")) {if let Ok(rel)=e.path().strip_prefix(root){files.insert(rel.to_path_buf(),());}}}}
        self.files=files.into_keys().collect();self.textures.clear();self.failed.clear();self.pending.clear();self.index=Index::default();
        let (tx,rx)=mpsc::channel();self.receiver=Some(rx);let ctx=ctx.clone();
        std::thread::spawn(move||{let _=tx.send(assets::build_index(&roots));ctx.request_repaint();});
        let relative=self.file.clone().filter(|p|self.resolve(p).is_some()).or_else(||self.files.iter().find(|p|p.ends_with("maingame_16.xml")).cloned()).or_else(||self.files.first().cloned());
        if let Some(p)=relative{self.open(p)}else{self.doc=None;self.file=None;self.loaded_source=None;self.selected=None;self.hidden.clear();self.drag_start=None;}
        self.status=format!("Found {} XML files. Indexing textures…",self.files.len());
        self.save_layers();
    }
    fn settings_path()->PathBuf{std::env::current_exe().unwrap_or_default().with_file_name("studio-layers.txt")}
    fn save_layers(&self){let mut text=format!("{}\n@base\t{}\n@font\t{}\n@atlas\t{}\n",self.root,if self.base_enabled{"1"}else{"0"},self.ui_font,self.atlas_height);for layer in &self.layers{text.push_str(&format!("{}\t{}\n",if layer.enabled{"1"}else{"0"},layer.root.display()));}let _=std::fs::write(Self::settings_path(),text);}
    fn load_layers(&mut self){if let Ok(text)=std::fs::read_to_string(Self::settings_path()){self.parse_layers(&text);}}
    fn parse_layers(&mut self,text:&str){let mut lines=text.lines();if let Some(root)=lines.next(){self.root=root.into();}self.layers.clear();self.base_enabled=true;for line in lines{if let Some((kind,value))=line.split_once('\t'){if kind=="@base"{self.base_enabled=value!="0";}else if kind=="@font"{if let Ok(v)=value.parse::<f32>(){if v.is_finite(){self.ui_font=v.clamp(10.0,24.0);}}}else if kind=="@atlas"{if let Ok(v)=value.parse::<f32>(){if v.is_finite(){self.atlas_height=v.clamp(90.0,1000.0);}}}else if kind=="0"||kind=="1"{self.layers.push(Layer{root:PathBuf::from(value),enabled:kind=="1"});}}}}
    fn open(&mut self,relative:PathBuf){
        if !self.edits.is_empty(){self.status="Export or discard current edits before opening another file.".into();return}
        let Some(path)=self.resolve(&relative) else{self.status="File not found in source layers.".into();return};
        match Document::read(&path){Ok(d)=>{self.status=format!("Opened {} · {} includes (not composed in v0.6)",path.display(),d.includes.len()); self.loaded_source=Some(path);self.doc=Some(d);self.file=Some(relative);self.tree_epoch+=1;self.selected=None;self.hidden.clear();self.undo.clear();self.redo.clear();},Err(e)=>self.status=format!("Could not open {}: {e}",path.display())}
    }
    fn checkpoint(&mut self){self.undo.push(self.edits.clone());if self.undo.len()>100{self.undo.remove(0);}self.redo.clear();}
    fn undo(&mut self){if let Some(e)=self.undo.pop(){self.redo.push(std::mem::replace(&mut self.edits,e));}}
    fn redo(&mut self){if let Some(e)=self.redo.pop(){self.undo.push(std::mem::replace(&mut self.edits,e));}}
    fn val(&self,i:usize,key:&str)->f32 {self.edits.get(&(i,key.into())).and_then(|s|s.parse().ok()).unwrap_or_else(||self.doc.as_ref().unwrap().nodes[i].num(key))}
    fn set(&mut self,i:usize,key:&str,v:f32){if !v.is_finite(){return}let value=format!("{:.3}",v).trim_end_matches('0').trim_end_matches('.').to_string();if self.doc.as_ref().unwrap().nodes[i].get(key)==value {self.edits.remove(&(i,key.into()));}else{self.edits.insert((i,key.into()),value);}}
    fn export(&mut self){
        let(Some(d),Some(relative))=(&self.doc,&self.file)else{return};
        let Some(folder)=rfd::FileDialog::new().set_title("Choose an output mod folder (gamedata will be added)").pick_folder()else{return};
        let output=folder.join("gamedata").join(relative);
        if std::iter::once(PathBuf::from(&self.root)).chain(self.layers.iter().map(|l|l.root.clone())).any(|r|output.starts_with(r)){self.status="Choose an output folder outside the source game/mod folders.".into();return}
        if output.exists() && rfd::MessageDialog::new().set_title("Replace exported XML?").set_description(output.display().to_string()).set_buttons(rfd::MessageButtons::YesNo).show()!=rfd::MessageDialogResult::Yes{return}
        let source=d.patched(&self.edits);
        let result=Document::parse(source.clone(),d.legacy,d.bom).and_then(|_|d.bytes(&source)).and_then(|b|{std::fs::create_dir_all(output.parent().unwrap()).and_then(|_|std::fs::write(&output,b)).map_err(|e|e.to_string())});
        self.status=match result{Ok(_)=>format!("Exported {}. Source remains unchanged; edits retained.",output.display()),Err(e)=>format!("Export failed: {e}")};
    }
    fn is_progress(&self,i:usize)->bool{let Some(d)=&self.doc else{return false};d.nodes[i].name.starts_with("progress_bar")||d.nodes.iter().any(|n|n.parent==Some(i)&&n.name=="progress")}
    fn texture_info(&self,i:usize)->Option<(PathBuf,Option<[f32;4]>,Color32,String,Option<PathBuf>)>{
        let d=self.doc.as_ref()?;let t=if d.nodes[i].name=="texture"{&d.nodes[i]}else{d.texture(i)?};let name=t.text.trim();
        let ch=|k:&str|t.get(k).parse::<u8>().unwrap_or(255);let tint=Color32::from_rgba_unmultiplied(ch("r"),ch("g"),ch("b"),ch("a"));
        if let Some(r)=self.index.regions.get(name){return Some((r.path.clone(),Some(r.rect),tint,name.into(),Some(r.descriptor.clone())))}
        for suffix in ["_e","_back"]{if let Some(r)=self.index.regions.get(&format!("{name}{suffix}")){return Some((r.path.clone(),Some(r.rect),tint,format!("{name} (preview piece: {suffix})"),Some(r.descriptor.clone())));}}
        let mut relative=PathBuf::from("textures").join(name.replace('\\',"/"));if relative.extension().is_none(){relative.set_extension("dds");}
        let path=self.resolve(&relative)?;let crop=if t.num("width")>0.0{Some([t.num("x"),t.num("y"),t.num("width"),t.num("height")])}else{None};
        Some((path,crop,tint,name.into(),None))
    }
    fn request_texture(&mut self,path:&Path,ctx:&egui::Context){
        if self.textures.contains_key(path)||self.failed.contains_key(path)||self.pending.contains(path)||self.pending.len()>=4{return}
        self.pending.insert(path.to_owned());let path=path.to_owned();let tx=self.texture_tx.clone();let ctx=ctx.clone();let generation=self.generation;
        std::thread::spawn(move||{let decoded=assets::decode(&path);let _=tx.send((generation,path,decoded));ctx.request_repaint();});
    }
    fn uv(crop:Option<[f32;4]>,size:Vec2)->Rect{crop.map(|c|Rect::from_min_max(Pos2::new(c[0]/size.x,c[1]/size.y),Pos2::new((c[0]+c[2])/size.x,(c[1]+c[3])/size.y))).unwrap_or(Rect::from_min_max(Pos2::ZERO,Pos2::new(1.0,1.0)))}
    fn paint_texture(&mut self,ctx:&egui::Context,painter:&egui::Painter,i:usize,rect:Rect)->bool{
        let Some((path,crop,tint,_,_))=self.texture_info(i)else{return false};self.request_texture(&path,ctx);
        let Some(tex)=self.textures.get(&path)else{return false};painter.image(tex.id(),rect,Self::uv(crop,tex.size_vec2()),tint);true
    }
    fn texture_explanation(&self,i:usize)->String{
        let Some(d)=&self.doc else{return String::new()};let n=&d.nodes[i];if self.texture_info(i).is_some(){return "Texture resolved from enabled sources. See the atlas inspector for its image and descriptor paths.".into()}
        let t=if n.name=="texture"{Some(n)}else{d.texture(i)};
        if let Some(t)=t{let name=t.text.trim();if name.is_empty(){return "Empty texture placeholder. Its image may be assigned by the engine or a script at runtime.".into()}
            if let Some((image,descriptor))=self.index.missing.get(name){return format!("Descriptor found: {}\nImage not found in enabled sources: {}",descriptor.display(),image.display())}
            if self.receiver.is_some(){return "Texture descriptors are still being indexed.".into()}
            return format!("Texture reference not resolved: {name}\nNo matching descriptor region, known state/frame piece, or direct image in the enabled layers. This can be a missing dependency or an editor limitation; it is not proof of a script assignment.");
        }
        if n.name=="text"{return "Text node: rendered with a font, not a UI texture region.".into()}
        if d.nodes.iter().any(|child|child.parent==Some(i)&&child.drawable()){return "Container/template with no texture of its own. Its child elements may supply the artwork.".into()}
        if d.nodes.iter().any(|child|child.parent==Some(i)&&child.name=="text"){return "Text/control with no texture declared here. Text can render without a background image.".into()}
        "No texture is declared on this element. It may be an untextured control, a template, or receive an image at runtime. The XML alone cannot identify which.".into()
    }
    fn atlas_panel(&mut self,ui:&mut egui::Ui,ctx:&egui::Context){
        ui.heading("Texture atlas / selected region");
        let Some(i)=self.selected else{ui.label("Select a textured element to inspect its atlas.");return};
        let Some((path,crop,_,name,descriptor))=self.texture_info(i)else{ui.label(self.texture_explanation(i));return};
        let key=(path.clone(),crop);if self.atlas_key.as_ref()!=Some(&key){self.atlas_key=Some(key);self.atlas_camera=AtlasCamera::default();self.region_pan=Vec2::ZERO;self.region_pointer=None;}
        ui.label(name);ui.small(path.display().to_string());if let Some(p)=descriptor{ui.small(format!("Descriptor: {}",p.display()));}
        self.request_texture(&path,ctx);
        if let Some(error)=self.failed.get(&path){ui.colored_label(Color32::LIGHT_RED,error);return}
        let Some(tex)=self.textures.get(&path)else{ui.label("Loading texture…");return};let size=tex.size_vec2();let texture=tex.id();
        let pane_bottom=ui.clip_rect().bottom();let width=((ui.available_width()-20.0)*0.5).max(30.0);
        ui.horizontal_top(|ui|{
            ui.vertical(|ui|{ui.set_width(width);ui.horizontal_wrapped(|ui|{ui.label(format!("Atlas · {} × {}",size.x,size.y));if ui.small_button("Fit atlas").clicked(){self.atlas_camera=AtlasCamera::default();}ui.small(format!("{:.2}× fit",self.atlas_camera.zoom));});
                let(view,_)=ui.allocate_exact_size(Vec2::new(width,(pane_bottom-ui.cursor().top()-4.0).max(12.0)),egui::Sense::hover());
                let c=crop.unwrap_or([0.0,0.0,size.x,size.y]);let focus=Vec2::new(c[0]+c[2]*0.5,c[1]+c[3]*0.5);
                let pointer=ctx.input(|i|i.pointer.clone());let hover=pointer.hover_pos().is_some_and(|p|view.contains(p));
                if hover{let wheel=ctx.input(|i|i.raw_scroll_delta.y);if wheel!=0.0{self.atlas_camera.zoom_by((wheel*0.005).exp());}}
                if hover&&pointer.button_pressed(egui::PointerButton::Middle){self.atlas_camera.pointer=pointer.press_origin();}
                if let Some(previous)=self.atlas_camera.pointer{if let Some(pos)=pointer.interact_pos(){self.atlas_camera.pan+=pos-previous;self.atlas_camera.pointer=Some(pos);}ctx.set_cursor_icon(egui::CursorIcon::Grabbing);}
                if !pointer.button_down(egui::PointerButton::Middle){self.atlas_camera.pointer=None;}
                let image=self.atlas_camera.image_rect(view,size,focus);let painter=ui.painter_at(view);checker(ui,view);painter.image(texture,image,Self::uv(None,size),Color32::WHITE);
                if crop.is_some(){let scale=image.width()/size.x;let region=Rect::from_min_size(image.min+Vec2::new(c[0],c[1])*scale,Vec2::new(c[2],c[3])*scale);painter.rect_stroke(region,0.0,Stroke::new(2.0_f32,Color32::GOLD),egui::StrokeKind::Outside);}
            });
            ui.vertical(|ui|{ui.set_width(width);ui.horizontal_wrapped(|ui|{ui.label("Selected region");if ui.checkbox(&mut self.region_native,"1:1 pixels").changed(){self.region_pan=Vec2::ZERO;self.region_pointer=None;}});
                let c=crop.unwrap_or([0.0,0.0,size.x,size.y]);ui.small(format!("x {} · y {} · {} × {}",c[0],c[1],c[2],c[3]));
                let(view,_)=ui.allocate_exact_size(Vec2::new(width,(pane_bottom-ui.cursor().top()-4.0).max(12.0)),egui::Sense::hover());
                if c[2]>0.0&&c[3]>0.0{let pointer=ctx.input(|i|i.pointer.clone());if self.region_native&&pointer.hover_pos().is_some_and(|p|view.contains(p))&&pointer.button_pressed(egui::PointerButton::Middle){self.region_pointer=pointer.press_origin();}
                    if let Some(previous)=self.region_pointer{if let Some(pos)=pointer.interact_pos(){self.region_pan+=pos-previous;self.region_pointer=Some(pos);}ctx.set_cursor_icon(egui::CursorIcon::Grabbing);}
                    if !pointer.button_down(egui::PointerButton::Middle){self.region_pointer=None;}
                    let dims=if self.region_native{Vec2::new(c[2],c[3])/ctx.pixels_per_point()}else{canvas_tools::contain(Vec2::new(c[2],c[3]),view.size())};
                    let image=Rect::from_center_size(view.center()+if self.region_native{self.region_pan}else{Vec2::ZERO},dims);checker(ui,view);let painter=ui.painter_at(view);painter.image(texture,image,Self::uv(crop,size),Color32::WHITE);painter.rect_stroke(image,0.0,Stroke::new(1.5_f32,Color32::GOLD),egui::StrokeKind::Inside);
                }
            });
        });
    }

    fn tree_node(&mut self,ui:&mut egui::Ui,i:usize,children:&[Vec<usize>],matches:&[bool]){
        if !matches[i]{return}let n=&self.doc.as_ref().unwrap().nodes[i];let changed=self.edits.keys().any(|(id,_)|*id==i);
        let extra=if !n.get("id").is_empty(){format!(" id=\"{}\"",n.get("id"))}else{String::new()};let label=format!("{}<{}{}>",if changed{"● "}else{""},n.name,extra);let selected=self.selected==Some(i);
        if children[i].is_empty(){if ui.selectable_label(selected,label).clicked(){self.selected=Some(i);}}
        else{let id=ui.make_persistent_id(("xml-node",self.tree_epoch,i));let mut state=egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(),id,self.tree_expand||i==0||!self.filter.is_empty());if !self.filter.is_empty(){state.set_open(true);}
            state.show_header(ui,|ui|{if ui.selectable_label(selected,label).clicked(){self.selected=Some(i);}}).body(|ui|{for &child in &children[i]{self.tree_node(ui,child,children,matches);}});
        }
    }
    fn layers_panel(&mut self,ui:&mut egui::Ui,ctx:&egui::Context){
        ui.heading("Mod layers");ui.small("Higher priority is lower in the list. Last enabled wins.");
        let dirty=!self.edits.is_empty();let mut changed=false;let mut action=None;
        ui.add_enabled_ui(!dirty,|ui|{
            egui::Frame::group(ui.style()).show(ui,|ui|{changed|=ui.checkbox(&mut self.base_enabled,"0  Base Anomaly").changed();ui.small(&self.root);});
            egui::ScrollArea::vertical().id_salt("layers-scroll").max_height(150.0).show(ui,|ui|{
                let len=self.layers.len();for (i,layer)in self.layers.iter_mut().enumerate(){egui::Frame::group(ui.style()).show(ui,|ui|{
                    ui.horizontal(|ui|{changed|=ui.checkbox(&mut layer.enabled,format!("{}",i+1)).changed();let name=layer.root.parent().filter(|_|layer.root.file_name().is_some_and(|n|n.eq_ignore_ascii_case("gamedata"))).unwrap_or(&layer.root).file_name().unwrap_or_default().to_string_lossy();ui.label(name).on_hover_text(layer.root.display().to_string());});
                    ui.horizontal(|ui|{if ui.add_enabled(i>0,egui::Button::new("Up / lower")).clicked(){action=Some((i,0));}if ui.add_enabled(i+1<len,egui::Button::new("Down / higher")).clicked(){action=Some((i,1));}if ui.small_button("Remove").clicked(){action=Some((i,2));}});
                });}
            });
            if ui.button("+ Add mod folder…").clicked(){if let Some(p)=rfd::FileDialog::new().pick_folder(){let root=if p.join("gamedata").is_dir(){p.join("gamedata")}else{p};if root.join("configs").is_dir()||root.join("textures").is_dir(){if !self.layers.iter().any(|l|l.root==root){self.layers.push(Layer{root,enabled:true});changed=true;}}else{self.status="Choose a mod folder or gamedata root containing configs or textures.".into();}}}
        });
        if dirty{ui.small("Export and discard edits before changing layers.");}
        if let Some((i,a))=action{match a{0=>self.layers.swap(i,i-1),1=>self.layers.swap(i,i+1),_=>{self.layers.remove(i);}}changed=true;}
        if changed{self.scan(ctx);}
    }
    fn canvas(&mut self,ui:&mut egui::Ui,ctx:&egui::Context){
        let output=OUTPUTS[self.ratio].1;
        let (viewport,response)=ui.allocate_exact_size(ui.available_size(),egui::Sense::click_and_drag());
        let pointer=ctx.input(|i|i.pointer.clone());let hover=pointer.hover_pos().is_some_and(|p|viewport.contains(p));
        if self.fit_requested{
            self.zoom=canvas_tools::fit_zoom(output,viewport.size(),ctx.pixels_per_point()).clamp(0.05,4.0);
            self.view_offset=(viewport.size()-canvas_tools::display_size(output,self.zoom,ctx.pixels_per_point()))*0.5;self.fit_requested=false;
        }
        if hover&&self.drag_start.is_none(){
            let (ctrl,factor,scroll)=ctx.input(|i|(i.modifiers.ctrl,i.zoom_delta(),i.raw_scroll_delta));
            if ctrl&&(factor-1.0).abs()>0.00001{let old=self.zoom;self.zoom=(old*factor).clamp(0.05,4.0);
                let anchor=pointer.hover_pos().unwrap()-viewport.min;self.view_offset=canvas_tools::anchored_offset(self.view_offset,anchor,old,self.zoom);
            }else if !ctrl&&scroll!=Vec2::ZERO{self.view_offset+=scroll;}
        }
        if hover&&pointer.button_pressed(egui::PointerButton::Middle){self.pan_pointer=pointer.press_origin();}
        if let Some(previous)=self.pan_pointer{if let Some(pos)=pointer.interact_pos(){self.view_offset+=pos-previous;self.pan_pointer=Some(pos);}ctx.set_cursor_icon(egui::CursorIcon::Grabbing);}
        if !pointer.button_down(egui::PointerButton::Middle){self.pan_pointer=None;}
        let size=canvas_tools::display_size(output,self.zoom,ctx.pixels_per_point());let scale=size.y/768.0;
        let canvas=Rect::from_min_size(viewport.min+self.view_offset,size);let painter=ui.painter_at(viewport);
        painter.rect_filled(viewport,0.0,Color32::from_rgb(13,16,20));
            painter.rect_filled(canvas,0.0,Color32::from_rgb(20,25,30));
            for n in 0..=16{let x=canvas.left()+size.x*n as f32/16.0;painter.line_segment([Pos2::new(x,canvas.top()),Pos2::new(x,canvas.bottom())],Stroke::new(0.5_f32,Color32::from_gray(38)));}
            for n in 0..=12{let y=canvas.top()+size.y*n as f32/12.0;painter.line_segment([Pos2::new(canvas.left(),y),Pos2::new(canvas.right(),y)],Stroke::new(0.5_f32,Color32::from_gray(38)));}
            let Some(d)=&self.doc else{return};
            let ids:Vec<usize>=d.nodes.iter().enumerate().filter(|(_,n)|n.drawable()).map(|(i,_)|i).collect();
            let mut rects=Vec::new();
            for &i in &ids {
                if self.hidden.contains(&i){continue}
                let mut x=self.val(i,"x");let mut y=self.val(i,"y");let mut parent=self.doc.as_ref().unwrap().nodes[i].parent;
                while let Some(p)=parent {let node=&self.doc.as_ref().unwrap().nodes[p];if node.drawable(){x+=self.val(p,"x");y+=self.val(p,"y");}parent=node.parent;}
                let w=self.val(i,"width");let h=self.val(i,"height");if w<=0.0||h<=0.0{continue}
                let sx=size.x/1024.0; let sy=size.y/768.0;
                let r=Rect::from_min_size(canvas.min+Vec2::new(x*sx,y*sy),Vec2::new(w*sx,h*sy)); rects.push((i,r));
                if !r.intersects(ui.clip_rect()){continue}
                let progress=self.is_progress(i);
                let rr=if progress{Rect::from_min_size(r.min,Vec2::new(r.width()*self.preview,r.height()))}else{r};
                let drawn=self.paint_texture(ctx,&painter,i,rr);
                if !drawn {let name=self.doc.as_ref().unwrap().nodes[i].name.clone();if progress{painter.rect_filled(rr,0.0,Color32::from_rgb(100,155,110));}
                    let text=self.doc.as_ref().unwrap().nodes.iter().find(|n|n.parent==Some(i)&&n.name=="text").map(|n|n.text.trim().to_string()).unwrap_or_default();
                    if !text.is_empty(){painter.text(r.center(),egui::Align2::CENTER_CENTER,text,egui::FontId::monospace(12.0*scale),Color32::LIGHT_GRAY);}else if self.bounds&&r.width()>45.0&&r.height()>13.0{painter.text(r.min+Vec2::splat(2.0),egui::Align2::LEFT_TOP,name,egui::FontId::monospace(9.0),Color32::from_gray(100));}
                }
                if self.bounds{painter.rect_stroke(r,0.0,Stroke::new(0.5_f32,Color32::from_rgba_unmultiplied(100,155,165,100)),egui::StrokeKind::Inside);}
            }
            if let Some(i)=self.selected {if let Some((_,r))=rects.iter().find(|(id,_)|*id==i){painter.rect_stroke(*r,0.0,Stroke::new(2.0_f32,Color32::from_rgb(245,190,85)),egui::StrokeKind::Outside);painter.rect_filled(Rect::from_center_size(r.max,Vec2::splat(9.0)),0.0,Color32::GOLD);}}
            if response.clicked_by(egui::PointerButton::Primary) {if let Some(pos)=response.interact_pointer_pos(){self.selected=rects.iter().rev().find(|(_,r)|r.contains(pos)&&r.width()<size.x*0.98).or_else(||rects.iter().rev().find(|(_,r)|r.contains(pos))).map(|(i,_)|*i);}}
            if response.drag_started_by(egui::PointerButton::Primary){if let Some(pos)=response.interact_pointer_pos(){let origin=ctx.input(|i|i.pointer.press_origin()).unwrap_or(pos);let hit=self.selected.and_then(|i|rects.iter().find(|(id,_)|*id==i)).filter(|(_,r)|r.expand(8.0).contains(origin)).or_else(||rects.iter().rev().find(|(_,r)|r.contains(origin)&&r.width()<size.x*0.98));
                if let Some(&(i,r))=hit {self.selected=Some(i);self.resize=r.max.distance(origin)<14.0;let keys=if self.resize{("width","height")}else{("x","y")};self.drag_start=Some(Drag::new(Vec2::new(self.val(i,keys.0),self.val(i,keys.1)),origin.to_vec2()));self.checkpoint();}
            }}
            if response.dragged_by(egui::PointerButton::Primary){if let (Some(i),Some(mut drag))=(self.selected,self.drag_start){
                if let Some(pos)=ctx.input(|input|input.pointer.interact_pos()){
                    let modifiers=ctx.input(|input|input.modifiers);
                    drag.update(pos.to_vec2(),Vec2::new(1024.0/size.x,768.0/size.y),modifiers.ctrl,modifiers.shift);
                    let value=drag.geometry(self.resize,modifiers.shift,modifiers.alt);
                    let keys=if self.resize{("width","height")}else{("x","y")};self.set(i,keys.0,value.x);self.set(i,keys.1,value.y);self.drag_start=Some(drag);
                }
            }}
            if let (Some(drag),Some(i))=(self.drag_start,self.selected){if let Some(axis)=drag.axis{if let Some((_,rect))=rects.iter().find(|(id,_)|*id==i){
                let anchor=if self.resize{rect.max}else{rect.min};let (a,b,label,color)=match axis{
                    canvas_tools::Axis::X=>(Pos2::new(viewport.left(),anchor.y),Pos2::new(viewport.right(),anchor.y),"X locked · horizontal",Color32::from_rgb(255,120,100)),
                    canvas_tools::Axis::Y=>(Pos2::new(anchor.x,viewport.top()),Pos2::new(anchor.x,viewport.bottom()),"Y locked · vertical",Color32::from_rgb(90,205,255)),
                };
                painter.line_segment([a,b],Stroke::new(1.5_f32,color));let label_pos=viewport.min+Vec2::splat(10.0);painter.rect_filled(Rect::from_min_size(label_pos-Vec2::splat(4.0),Vec2::new(200.0,25.0)),3.0,Color32::from_black_alpha(210));painter.text(label_pos,egui::Align2::LEFT_TOP,label,egui::FontId::proportional(14.0),color);
            }}}
            if response.drag_stopped_by(egui::PointerButton::Primary){self.drag_start=None;}
    }
}
impl eframe::App for App {
    fn update(&mut self,ctx:&egui::Context,_:&mut eframe::Frame){
        ctx.style_mut(|style|{for(text_style,size)in [(egui::TextStyle::Body,self.ui_font),(egui::TextStyle::Button,self.ui_font),(egui::TextStyle::Small,(self.ui_font-2.0).max(9.0)),(egui::TextStyle::Heading,self.ui_font+4.0),(egui::TextStyle::Monospace,self.ui_font)]{style.text_styles.insert(text_style,egui::FontId::proportional(size));}style.spacing.interact_size.y=self.ui_font+6.0;});

        if ctx.input(|i|i.viewport().close_requested()) && !self.edits.is_empty() {
            if rfd::MessageDialog::new().set_title("Close with pending edits?").set_description("Edits are held in memory. Export first to keep them. Close anyway?").set_buttons(rfd::MessageButtons::YesNo).show()!=rfd::MessageDialogResult::Yes {ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);}
        }
        while let Ok((generation,path,decoded))=self.texture_rx.try_recv(){if generation!=self.generation{continue}self.pending.remove(&path);match decoded{Ok(image)=>{let tex=ctx.load_texture(path.to_string_lossy(),image,egui::TextureOptions::LINEAR);self.textures.insert(path,tex);},Err(e)=>{self.failed.insert(path,e);}}}

        if let Some(rx)=&self.receiver {if let Ok(index)=rx.try_recv(){self.status=format!("Ready · {} texture regions indexed",index.regions.len());self.index=index;self.receiver=None;}}
        if ctx.input(|i|i.modifiers.ctrl&&i.key_pressed(egui::Key::Z)){self.undo();}
        if ctx.input(|i|i.modifiers.ctrl&&i.key_pressed(egui::Key::Y)){self.redo();}
        egui::TopBottomPanel::top("toolbar").show(ctx,|ui|{
            ui.horizontal_wrapped(|ui|{ui.heading("ANOMALY / UI STUDIO");ui.label("v0.6");ui.separator();if ui.button("Export mod…").clicked(){self.export();}if ui.button("Undo").clicked(){self.undo()}if ui.button("Redo").clicked(){self.redo()}if ui.button("Discard edits").clicked(){self.checkpoint();self.edits.clear();}ui.label(format!("{} changed attributes",self.edits.len()));});
            ui.add_enabled_ui(self.edits.is_empty(),|ui|{ui.horizontal(|ui|{ui.label("Base gamedata");ui.add(egui::TextEdit::singleline(&mut self.root).desired_width(490.0));if ui.button("Browse…").clicked(){if let Some(p)=rfd::FileDialog::new().pick_folder(){self.root=p.display().to_string();self.scan(ctx);}}if ui.button("Rescan").clicked(){self.scan(ctx)}});});
            ui.horizontal_wrapped(|ui|{egui::ComboBox::from_id_salt("ratio").selected_text(OUTPUTS[self.ratio].0).show_ui(ui,|ui|{for(i,(label,_))in OUTPUTS.iter().enumerate(){ui.selectable_value(&mut self.ratio,i,*label);}});
                ui.add(egui::Slider::new(&mut self.zoom,0.05..=4.0).logarithmic(true).text("Zoom"));
                if ui.button("1× pixels").on_hover_text("One output pixel per physical display pixel; independent of window size").clicked(){self.zoom=1.0;}
                if ui.button("Fit view").clicked(){self.fit_requested=true;}
                ui.checkbox(&mut self.bounds,"Bounds");ui.checkbox(&mut self.show_xml,"XML changes");ui.separator();for(stem,label)in [("maingame","HUD"),("ui_mm_main","Main menu"),("ui_options","Options")]{if ui.add_enabled(self.edits.is_empty(),egui::Button::new(label)).clicked(){let suffix=if self.ratio<3{"16"}else{"21"};let preferred=PathBuf::from(format!("configs/ui/{stem}_{suffix}.xml"));let fallback=PathBuf::from(format!("configs/ui/{stem}_16.xml"));let path=if self.resolve(&preferred).is_some(){preferred}else{fallback};self.open(path);}}});
        });
        egui::TopBottomPanel::bottom("status").show(ctx,|ui|{
            ui.add(egui::Label::new(&self.status).truncate()).on_hover_text(&self.status);
            ui.horizontal(|ui|{
                let available=(ui.available_width()-140.0).max(30.0);
                ui.allocate_ui(Vec2::new(available,self.ui_font+6.0),|ui|{ui.add(egui::Label::new(egui::RichText::new("Ctrl: axis lock · Ctrl+wheel: zoom · Middle-drag: pan · Shift: fine · Alt: keep proportions · XML preview is approximate").small()).truncate());});
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui|{if ui.add(egui::DragValue::new(&mut self.ui_font).range(10.0..=24.0).speed(0.2).suffix(" px")).on_hover_text("Editor font size").changed(){self.save_layers();}ui.label("Font");});
            });
        });
        egui::SidePanel::left("tree").resizable(true).default_width(300.0).show(ctx,|ui|{
            ui.separator();egui::CollapsingHeader::new(format!("Layers · {} enabled",self.layers.iter().filter(|l|l.enabled).count()+usize::from(self.base_enabled))).id_salt("source-layers").show(ui,|ui|self.layers_panel(ui,ctx));ui.separator();
            ui.heading("XML files");
            ui.add(egui::TextEdit::singleline(&mut self.file_filter).hint_text("Search XML filenames…"));
            let query=self.file_filter.to_lowercase();let files:Vec<_>=self.files.iter().filter(|p|p.strip_prefix("configs/ui").unwrap_or(p).to_string_lossy().to_lowercase().contains(&query)).cloned().collect();
            let row_height=ui.spacing().interact_size.y;let list_height=row_height*10.0+ui.spacing().item_spacing.y*9.0;
            egui::Frame::group(ui.style()).show(ui,|ui|{egui::ScrollArea::vertical().id_salt("xml-files").min_scrolled_height(list_height).max_height(list_height).auto_shrink([false,false]).show_rows(ui,row_height,files.len(),|ui,rows|{
                for row in rows{let p=&files[row];let label=p.strip_prefix("configs/ui").unwrap_or(p).display().to_string();if ui.add_enabled(self.edits.is_empty(),egui::Button::new(label).selected(self.file.as_ref()==Some(p)).frame(false).wrap_mode(egui::TextWrapMode::Truncate).min_size(Vec2::new(ui.available_width(),row_height))).clicked(){self.open(p.clone());}}
            });});
            ui.small(format!("{} / {} files",files.len(),self.files.len()));
            if let Some(p)=&self.loaded_source{ui.small("Loaded from:");ui.small(p.display().to_string());}
            ui.separator();ui.heading("XML structure");
            ui.add(egui::TextEdit::singleline(&mut self.filter).hint_text("Search elements…"));ui.horizontal(|ui|{if ui.small_button("Expand all").clicked(){self.tree_expand=true;self.tree_epoch+=1;}if ui.small_button("Collapse all").clicked(){self.tree_expand=false;self.tree_epoch+=1;}});
            egui::Frame::group(ui.style()).show(ui,|ui|{ui.set_min_width(ui.available_width());
            egui::ScrollArea::vertical().id_salt("xml-tree").auto_shrink([false,false]).show(ui,|ui|{if let Some(d)=&self.doc{let mut children=vec![Vec::new();d.nodes.len()];let mut roots=Vec::new();let filter=self.filter.to_lowercase();let mut matches:Vec<_>=d.nodes.iter().map(|n|filter.is_empty()||n.name.to_lowercase().contains(&filter)||n.get("id").to_lowercase().contains(&filter)).collect();for(i,n)in d.nodes.iter().enumerate(){if let Some(p)=n.parent{children[p].push(i)}else{roots.push(i)}}for i in (0..d.nodes.len()).rev(){if matches[i]{if let Some(p)=d.nodes[i].parent{matches[p]=true;}}}for i in roots{self.tree_node(ui,i,&children,&matches);}}});
            });
        });
        egui::SidePanel::right("properties").default_width(250.0).resizable(true).show(ctx,|ui|{
            ui.heading("Properties");if let Some(i)=self.selected {let name=self.doc.as_ref().unwrap().nodes[i].name.clone();ui.label(name);if self.is_progress(i){ui.label("Sample fill (preview only)");ui.add(egui::Slider::new(&mut self.preview,0.0..=1.0));}let mut visible=!self.hidden.contains(&i);if ui.checkbox(&mut visible,"Visible in preview").changed(){if visible{self.hidden.remove(&i);}else{self.hidden.insert(i);}}ui.separator();for key in ["x","y","width","height"]{let exists=self.doc.as_ref().unwrap().nodes[i].attrs.contains_key(key);if !exists{continue}let mut value=self.val(i,key);ui.horizontal(|ui|{ui.label(key);let r=ui.add(egui::DragValue::new(&mut value).speed(0.25));if r.changed(){if !r.dragged()||r.drag_started(){self.checkpoint();}self.set(i,key,value);}});}ui.separator();if let Some(t)=self.doc.as_ref().unwrap().texture(i){ui.label("Texture");ui.monospace(t.text.trim());}else{ui.small(self.texture_explanation(i));}ui.collapsing("All attributes",|ui|{for(k,a)in &self.doc.as_ref().unwrap().nodes[i].attrs{ui.label(format!("{k} = {}",a.value));}});}else{ui.label("Select an element in the tree or canvas.");}ui.separator();ui.small("Drag: move · Gold corner: resize\nCtrl/Strg: lock dominant X/Y axis\nCtrl+wheel: zoom · Middle-drag: pan\nShift: 10× finer movement\nAlt: keep resize aspect ratio\nCtrl+Z / Ctrl+Y: undo / redo");
            if !self.failed.is_empty(){ui.collapsing(format!("{} texture errors",self.failed.len()),|ui|{for(p,e)in &self.failed{ui.small(p.display().to_string());ui.colored_label(Color32::LIGHT_RED,e);}});}
        });
        if self.show_xml {egui::TopBottomPanel::bottom("xml").resizable(true).default_height(150.0).show(ctx,|ui|{egui::ScrollArea::vertical().show(ui,|ui|{if let Some(d)=&self.doc{for((i,k),v)in &self.edits{ui.monospace(format!("{} #{i}: {k}  {} → {v}",d.nodes[*i].name,d.nodes[*i].get(k)));}}});});}
        egui::CentralPanel::default().show(ctx,|ui|{
            let area=ui.available_rect_before_wrap();let height=self.atlas_height.clamp(90.0,(area.height()-110.0).max(90.0));let split_y=(area.bottom()-height-8.0).max(area.top()+40.0);
            let canvas_rect=Rect::from_min_max(area.min,Pos2::new(area.right(),split_y));
            let atlas_rect=Rect::from_min_max(Pos2::new(area.left(),split_y+8.0),area.max);
            let mut canvas_ui=ui.new_child(egui::UiBuilder::new().id_salt("canvas-pane").max_rect(canvas_rect));canvas_ui.set_clip_rect(canvas_rect);self.canvas(&mut canvas_ui,ctx);
            let mut atlas_ui=ui.new_child(egui::UiBuilder::new().id_salt("atlas-pane").max_rect(atlas_rect));atlas_ui.set_clip_rect(atlas_rect);
            self.atlas_panel(&mut atlas_ui,ctx);
            let divider=Rect::from_min_max(Pos2::new(area.left(),split_y),Pos2::new(area.right(),split_y+8.0));let response=ui.interact(divider,ui.make_persistent_id("atlas-divider"),egui::Sense::drag());
            if response.hovered()||response.dragged(){ctx.set_cursor_icon(egui::CursorIcon::ResizeVertical);}
            let pointer=ctx.input(|i|i.pointer.clone());
            if pointer.primary_pressed()&&pointer.press_origin().is_some_and(|p|divider.contains(p)){self.atlas_drag=true;}
            if self.atlas_drag||response.dragged(){if let Some(pos)=pointer.interact_pos(){self.atlas_height=(area.bottom()-pos.y-4.0).clamp(90.0,(area.height()-110.0).max(90.0));}ctx.set_cursor_icon(egui::CursorIcon::ResizeVertical);}
            if (self.atlas_drag&&pointer.any_released())||response.drag_stopped(){self.atlas_drag=false;self.save_layers();}
            let stroke=if response.dragged()||self.atlas_drag{ui.visuals().widgets.active.fg_stroke}else if response.hovered(){ui.visuals().widgets.hovered.fg_stroke}else{ui.visuals().widgets.noninteractive.bg_stroke};
            ui.painter().hline(divider.x_range(),divider.center().y,stroke);ui.advance_cursor_after_rect(area);
        });
    }
}
fn checker(ui:&egui::Ui,rect:Rect){let painter=ui.painter_at(rect);let tile=12.0;for y in 0..=(rect.height()/tile)as usize{for x in 0..=(rect.width()/tile)as usize{let r=Rect::from_min_size(rect.min+Vec2::new(x as f32*tile,y as f32*tile),Vec2::splat(tile));painter.rect_filled(r,0.0,Color32::from_gray(if(x+y)%2==0{44}else{66}));}}}
fn main()->eframe::Result {
    let args:Vec<_>=std::env::args().collect();
    if let Some(pos)=args.iter().position(|a|a=="--check-mod-assets"){
        let mut lines=Vec::new();let mut count=0;let mut errors=0;
        for root in &args[pos+1..]{for e in walkdir::WalkDir::new(root).into_iter().filter_map(Result::ok){if e.file_type().is_file()&&e.path().extension().is_some_and(|s|s.eq_ignore_ascii_case("dds")){count+=1;if let Err(error)=assets::decode(e.path()){errors+=1;lines.push(format!("{}: {error}",e.path().display()));}}}}
        lines.insert(0,format!("DDS decoded: {} / {}. Failures: {}",count-errors,count,errors));std::fs::write("asset-validation.txt",lines.join("\n")).unwrap();if errors>0{std::process::exit(1)}return Ok(())
    }
    if std::env::args().any(|a|a=="--validate-game") {let root=PathBuf::from(App::default().root);for name in ["maingame.xml","maingame_16.xml","ui_mm_main_16.xml","ui_options_16.xml"]{let d=Document::read(&root.join("configs/ui").join(name)).expect(name);assert_eq!(d.bytes(&d.patched(&Edits::new())).unwrap(),std::fs::read(root.join("configs/ui").join(name)).unwrap());}return Ok(())}
    let icon=image::load_from_memory(include_bytes!("../assets/app.ico")).expect("Bundled app icon").into_rgba8();
    let icon=egui::IconData{width:icon.width(),height:icon.height(),rgba:icon.into_raw()};
    eframe::run_native("Anomaly UI Studio",eframe::NativeOptions{viewport:egui::ViewportBuilder::default().with_inner_size([1440.0,900.0]).with_icon(icon),..Default::default()},Box::new(|cc|{cc.egui_ctx.set_visuals(egui::Visuals::dark());let mut app=App::default();app.load_layers();if Path::new(&app.root).is_dir()||!app.layers.is_empty(){app.scan(&cc.egui_ctx);}Ok(Box::new(app))}))
}

#[cfg(test)]mod app_tests{
    use super::*;
    #[test]fn atlas_wheel_middle_pan_and_native_region_are_separate_from_xml(){
        let mut app=App::default();let ctx=egui::Context::default();let path=PathBuf::from("fixture.dds");
        app.doc=Some(Document::parse("<w><box><texture>fixture</texture></box></w>".into(),false,false).unwrap());app.selected=Some(1);
        app.index.regions.insert("fixture".into(),assets::Region{path:path.clone(),rect:[700.0,100.0,100.0,100.0],descriptor:PathBuf::from("fixture.xml")});
        app.textures.insert(path,ctx.load_texture("fixture",egui::ColorImage::new([1000,1000],Color32::WHITE),Default::default()));
        let mut draw=|events|ctx.run(egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(1000.0,600.0))),events,..Default::default()},|ctx|{egui::CentralPanel::default().show(ctx,|ui|app.atlas_panel(ui,ctx));});
        let p=Pos2::new(200.0,250.0);draw(vec![egui::Event::PointerMoved(p)]);
        draw(vec![egui::Event::MouseWheel{unit:egui::MouseWheelUnit::Point,delta:Vec2::new(0.0,80.0),modifiers:egui::Modifiers::NONE}]);
        draw(vec![egui::Event::PointerButton{pos:p,button:egui::PointerButton::Middle,pressed:true,modifiers:egui::Modifiers::NONE}]);
        draw(vec![egui::Event::PointerMoved(p+Vec2::new(40.0,20.0))]);
        assert!(app.atlas_camera.zoom>1.0&&app.atlas_camera.focused);assert_eq!(app.atlas_camera.pan,Vec2::new(40.0,20.0));assert_eq!(app.zoom,1.0);assert!(app.edits.is_empty());
        app.region_native=true;let output=ctx.run(egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(1000.0,600.0))),..Default::default()},|ctx|{egui::CentralPanel::default().show(ctx,|ui|app.atlas_panel(ui,ctx));});
        let image=output.shapes.iter().rev().find_map(|s|if let egui::Shape::Mesh(m)=&s.shape{Some(m.calc_bounds())}else{None}).unwrap();assert_eq!(image.size(),Vec2::splat(100.0));
    }
    #[test]fn texture_diagnostics_do_not_assume_every_container_is_scripted(){let mut app=App::default();app.doc=Some(Document::parse("<w><container><child x='0' width='4' height='4'/></container><missing><texture>unknown_id</texture></missing></w>".into(),false,false).unwrap());assert!(app.texture_explanation(1).contains("Container"));assert!(app.texture_explanation(3).contains("not proof of a script assignment"));}
    fn frame(app:&mut App,ctx:&egui::Context,events:Vec<egui::Event>,modifiers:egui::Modifiers)->egui::FullOutput{
        ctx.run(egui::RawInput{screen_rect:Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(1000.0,800.0))),events,modifiers,..Default::default()},|ctx|{egui::CentralPanel::default().show(ctx,|ui|app.canvas(ui,ctx));})
    }
    #[test]fn ctrl_drag_paints_axis_guide(){
        let mut app=App::default();app.doc=Some(Document::parse("<w><box x='100' y='100' width='100' height='100'/></w>".into(),false,false).unwrap());let ctx=egui::Context::default();let p=Pos2::new(220.0,170.0);
        frame(&mut app,&ctx,vec![egui::Event::PointerMoved(p)],egui::Modifiers::CTRL);
        frame(&mut app,&ctx,vec![egui::Event::PointerButton{pos:p,button:egui::PointerButton::Primary,pressed:true,modifiers:egui::Modifiers::CTRL}],egui::Modifiers::CTRL);
        frame(&mut app,&ctx,vec![egui::Event::PointerMoved(p+Vec2::new(40.0,2.0))],egui::Modifiers::CTRL);
        let output=frame(&mut app,&ctx,vec![egui::Event::PointerMoved(p+Vec2::new(70.0,4.0))],egui::Modifiers::CTRL);
        assert_eq!(app.drag_start.unwrap().axis,Some(canvas_tools::Axis::X));
        assert!(output.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.job.text.contains("X locked"))));assert_eq!(app.val(1,"y"),100.0);
    }
    #[test]fn canvas_ctrl_wheel_and_middle_pan_do_not_edit_xml(){
        let mut app=App::default();let ctx=egui::Context::default();let p=Pos2::new(300.0,200.0);
        frame(&mut app,&ctx,vec![egui::Event::PointerMoved(p)],egui::Modifiers::NONE);
        frame(&mut app,&ctx,vec![egui::Event::MouseWheel{unit:egui::MouseWheelUnit::Point,delta:Vec2::new(0.0,80.0),modifiers:egui::Modifiers::CTRL}],egui::Modifiers::CTRL);
        assert!(app.zoom>1.0);assert!(app.view_offset.x<0.0);assert_eq!(app.ui_font,14.0);
        let before=app.view_offset;
        frame(&mut app,&ctx,vec![egui::Event::PointerButton{pos:p,button:egui::PointerButton::Middle,pressed:true,modifiers:egui::Modifiers::NONE}],egui::Modifiers::NONE);
        frame(&mut app,&ctx,vec![egui::Event::PointerMoved(p+Vec2::new(50.0,30.0))],egui::Modifiers::NONE);
        assert!((app.view_offset-before-Vec2::new(50.0,30.0)).length()<0.01);assert!(app.edits.is_empty());assert!(app.undo.is_empty());
    }
    #[test]fn base_toggle_removes_fallback_and_old_settings_still_load(){
        let mut app=App::default();app.parse_layers("base\n1\tmod-a\n0\tmod-b\n");assert_eq!(app.roots(),vec![PathBuf::from("base"),PathBuf::from("mod-a")]);
        app.parse_layers("base\n@base\t0\n1\tmod-a\n0\tmod-b\n");assert_eq!(app.roots(),vec![PathBuf::from("mod-a")]);
        app.layers[0].enabled=false;assert!(app.roots().is_empty());
    }
}
