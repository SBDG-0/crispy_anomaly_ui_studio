use egui::Vec2;

pub const OUTPUTS:[(&str,[f32;2]);5]=[
    ("16:9 · 1920 × 1080",[1920.0,1080.0]),
    ("16:9 · 2560 × 1440",[2560.0,1440.0]),
    ("16:9 · 3840 × 2160",[3840.0,2160.0]),
    ("21:9 · 2560 × 1080",[2560.0,1080.0]),
    ("21:9 · 3440 × 1440",[3440.0,1440.0]),
];
// Zoom is defined in physical display pixels, independently of the viewport.
pub fn display_size(output:[f32;2],zoom:f32,pixels_per_point:f32)->Vec2{
    Vec2::new(output[0],output[1])*zoom/pixels_per_point
}
pub fn fit_zoom(output:[f32;2],viewport:Vec2,pixels_per_point:f32)->f32{
    ((viewport.x-20.0).max(1.0)/output[0]).min((viewport.y-20.0).max(1.0)/output[1])*pixels_per_point
}
pub fn anchored_offset(offset:Vec2,anchor:Vec2,old_zoom:f32,new_zoom:f32)->Vec2{
    anchor-(anchor-offset)*(new_zoom/old_zoom)
}
pub fn contain(image:Vec2,space:Vec2)->Vec2{
    if image.x<=0.0||image.y<=0.0{return Vec2::ZERO}
    image*(space.x.max(1.0)/image.x).min(space.y.max(1.0)/image.y)
}
#[derive(Clone,Copy,PartialEq,Debug)]pub enum Axis{X,Y}
#[derive(Clone,Copy)]
pub struct Drag{
    pub start:Vec2,pub last_pointer:Vec2,pub displacement:Vec2,pub axis:Option<Axis>,
    previous_displacement:Vec2,units_per_point:Vec2,size_anchor:Vec2,delta_anchor:Vec2,aspect_active:bool,resize_initialized:bool,last_geometry:Vec2,
}
impl Drag{
    pub fn new(start:Vec2,pointer:Vec2)->Self{Self{start,last_pointer:pointer,displacement:Vec2::ZERO,axis:None,previous_displacement:Vec2::ZERO,units_per_point:Vec2::splat(1.0),size_anchor:start,delta_anchor:Vec2::ZERO,aspect_active:false,resize_initialized:false,last_geometry:start}}
    pub fn update(&mut self,pointer:Vec2,units_per_point:Vec2,ctrl:bool,shift:bool){
        self.previous_displacement=self.displacement;self.units_per_point=units_per_point;let raw=pointer-self.last_pointer;self.last_pointer=pointer;
        if !ctrl{self.axis=None;}else if self.axis.is_none()&&raw.length_sq()>0.0001{
            self.axis=Some(if raw.x.abs()>=raw.y.abs(){Axis::X}else{Axis::Y});
        }
        let mut delta=raw*units_per_point*if shift{0.1}else{1.0};
        match self.axis{Some(Axis::X)=>delta.y=0.0,Some(Axis::Y)=>delta.x=0.0,None=>{}}
        self.displacement+=delta;
    }
    pub fn geometry(&mut self,resize:bool,shift:bool,keep_aspect:bool)->Vec2{
        let quantum=if shift{0.1}else{1.0};let round=|x:f32|(x/quantum).round()*quantum;
        if !resize{return Vec2::new(if self.displacement.x==0.0{self.start.x}else{round(self.start.x+self.displacement.x)},if self.displacement.y==0.0{self.start.y}else{round(self.start.y+self.displacement.y)})}
        if !self.resize_initialized{self.resize_initialized=true;self.aspect_active=keep_aspect;}
        else if self.aspect_active!=keep_aspect{
            self.size_anchor=self.last_geometry;self.delta_anchor=self.previous_displacement;self.aspect_active=keep_aspect;
        }
        let delta=self.displacement-self.delta_anchor;let anchor=self.size_anchor;
        let result=if keep_aspect&&anchor.x>0.0&&anchor.y>0.0{
            // Project the handle motion onto its screen-space diagonal. Unlike
            // choosing the largest axis each frame, this is continuous across
            // direction changes and works with Anomaly's non-square UI units.
            let diagonal=anchor/self.units_per_point;let motion=delta/self.units_per_point;
            let factor=(1.0+motion.dot(diagonal)/diagonal.length_sq()).max((1.0/anchor.x).max(1.0/anchor.y));
            anchor*factor
        }else{Vec2::new(if delta.x==0.0{anchor.x}else{round(anchor.x+delta.x).max(1.0)},if delta.y==0.0{anchor.y}else{round(anchor.y+delta.y).max(1.0)})};
        self.last_geometry=result;result
    }

}
#[cfg(test)]mod tests{
    use super::*;
    #[test]fn zoom_is_physical_and_independent_of_viewport(){for(_,output)in OUTPUTS{let s=display_size(output,1.0,1.5);assert_eq!(s*1.5,Vec2::from(output));}assert_eq!(display_size([1920.0,1080.0],0.5,1.0),Vec2::new(960.0,540.0));}
    #[test]fn fit_is_explicit_zoom(){let output=[3440.0,1440.0];let v=Vec2::new(900.0,600.0);let z=fit_zoom(output,v,1.5);let s=display_size(output,z,1.5);assert!(s.x<=v.x&&s.y<=v.y);}
    #[test]fn ctrl_locks_axis_until_released(){let mut d=Drag::new(Vec2::ZERO,Vec2::ZERO);d.update(Vec2::new(20.0,3.0),Vec2::splat(1.0),true,false);d.update(Vec2::new(25.0,70.0),Vec2::splat(1.0),true,false);assert_eq!(d.displacement,Vec2::new(25.0,0.0));d.update(Vec2::new(30.0,80.0),Vec2::splat(1.0),false,false);assert_eq!(d.displacement,Vec2::new(30.0,10.0));}
    #[test]fn shift_precision_is_incremental(){let mut d=Drag::new(Vec2::ZERO,Vec2::ZERO);d.update(Vec2::new(10.0,0.0),Vec2::splat(1.0),false,false);d.update(Vec2::new(20.0,0.0),Vec2::splat(1.0),false,true);assert_eq!(d.geometry(false,true,false).x,11.0);}
    #[test]fn alt_resize_preserves_ratio(){let mut d=Drag::new(Vec2::new(138.0,16.0),Vec2::ZERO);d.update(Vec2::new(35.0,2.0),Vec2::splat(1.0),false,false);let s=d.geometry(true,false,true);assert!((s.x/s.y-138.0/16.0).abs()<0.0001);let mut d=Drag::new(Vec2::new(100.0,50.0),Vec2::ZERO);d.displacement=Vec2::new(-1000.0,-1000.0);let s=d.geometry(true,false,true);assert!(s.x>=1.0&&s.y>=1.0);assert_eq!(s.x/s.y,2.0);}
    #[test]fn pointer_zoom_keeps_same_image_point(){let offset=Vec2::new(-100.0,30.0);let anchor=Vec2::new(300.0,200.0);let new=anchored_offset(offset,anchor,0.5,1.0);assert_eq!((anchor-offset)/0.5,(anchor-new)/1.0);assert_eq!(anchored_offset(new,anchor,1.0,0.5),offset);}
    #[test]fn atlas_images_grow_with_pane(){let image=Vec2::new(2048.0,2048.0);let small=contain(image,Vec2::new(500.0,100.0));let big=contain(image,Vec2::new(500.0,400.0));assert_eq!(small,Vec2::splat(100.0));assert_eq!(big,Vec2::splat(400.0));}
}

#[derive(Clone,Copy,Debug)]
pub struct AtlasCamera{pub zoom:f32,pub pan:Vec2,pub focused:bool,pub pointer:Option<egui::Pos2>}
impl Default for AtlasCamera{fn default()->Self{Self{zoom:1.0,pan:Vec2::ZERO,focused:false,pointer:None}}}
impl AtlasCamera{
    pub fn zoom_by(&mut self,factor:f32){self.zoom=(self.zoom*factor).clamp(0.1,128.0);self.focused=true;}
    pub fn image_rect(&self,view:egui::Rect,image:Vec2,region_center:Vec2)->egui::Rect{
        let scale=(view.width()/image.x).min(view.height()/image.y)*self.zoom;
        let focus=if self.focused{region_center}else{image*0.5};
        egui::Rect::from_min_size(view.center()+self.pan-focus*scale,image*scale)
    }
}
#[cfg(test)]mod regression_tests{
    use super::*;
    #[test]fn proportional_resize_is_continuous_across_opposing_axis_crossover(){
        let mut d=Drag::new(Vec2::new(100.0,100.0),Vec2::ZERO);d.update(Vec2::new(20.0,-19.9),Vec2::splat(1.0),false,false);let a=d.geometry(true,false,true);
        d.update(Vec2::new(20.0,-20.1),Vec2::splat(1.0),false,false);let b=d.geometry(true,false,true);assert!((a-b).length()<0.3);
    }
    #[test]fn toggling_alt_does_not_jump_without_pointer_motion(){let mut d=Drag::new(Vec2::new(100.0,50.0),Vec2::ZERO);d.update(Vec2::new(20.0,5.0),Vec2::splat(1.0),false,false);let a=d.geometry(true,false,false);d.update(Vec2::new(20.0,5.0),Vec2::splat(1.0),false,false);let b=d.geometry(true,false,true);assert_eq!(a,b);d.update(Vec2::new(30.0,10.0),Vec2::splat(1.0),false,false);let c=d.geometry(true,false,true);assert!((c.x/c.y-a.x/a.y).abs()<0.00001);}
    #[test]fn atlas_zoom_focuses_highlight_and_pan_is_independent(){let view=egui::Rect::from_min_size(egui::Pos2::ZERO,Vec2::new(400.0,300.0));let image=Vec2::splat(1000.0);let focus=Vec2::new(800.0,200.0);let mut camera=AtlasCamera::default();camera.zoom_by(2.0);let rect=camera.image_rect(view,image,focus);let projected=rect.min+focus*(rect.width()/image.x);assert!((projected-view.center()).length()<0.01);camera.pan=Vec2::new(20.0,-30.0);let next=camera.image_rect(view,image,focus);assert_eq!(next.min-rect.min,camera.pan);}
}
