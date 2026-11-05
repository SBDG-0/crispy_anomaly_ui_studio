use crate::document::Document;
use std::{collections::{BTreeMap, HashMap},path::{Path,PathBuf}};

#[derive(Clone,Debug)]
pub struct Region {pub path:PathBuf,pub rect:[f32;4],pub descriptor:PathBuf}
#[derive(Default)]
pub struct Index {pub regions:HashMap<String,Region>,pub warnings:Vec<String>,pub missing:HashMap<String,(PathBuf,PathBuf)>}

pub fn resolve(roots:&[PathBuf],relative:&Path)->Option<PathBuf>{roots.iter().rev().map(|r|r.join(relative)).find(|p|p.is_file())}

pub fn build_index(roots:&[PathBuf])->Index {
    // Resolve the virtual descriptor files first; an overridden file replaces its
    // predecessor completely, including IDs removed by the higher priority mod.
    let mut descriptors=BTreeMap::new();
    for root in roots {for e in walkdir::WalkDir::new(root.join("configs/ui/textures_descr")).sort_by_file_name().into_iter().filter_map(Result::ok){
        if e.file_type().is_file()&&e.path().extension().is_some_and(|s|s.eq_ignore_ascii_case("xml")){if let Ok(rel)=e.path().strip_prefix(root){descriptors.insert(rel.to_string_lossy().to_lowercase(),e.path().to_owned());}}
    }}
    let mut index=Index::default();
    // Definitions from higher layers win, then descriptor filename order.
    let mut paths:Vec<_>=descriptors.into_values().collect();
    paths.sort_by_key(|p|(roots.iter().rposition(|r|p.starts_with(r)).unwrap_or(0),p.clone()));
    for path in paths {let d=match Document::read(&path){Ok(d)=>d,Err(e)=>{index.warnings.push(format!("{}: {e}",path.display()));continue}};
        for n in &d.nodes {if n.name!="texture"||n.get("id").is_empty(){continue}
            let Some(parent)=n.parent else{continue};let file=&d.nodes[parent];if file.name!="file"{continue}
            let mut relative=PathBuf::from("textures").join(file.get("name").replace('\\',"/"));if relative.extension().is_none(){relative.set_extension("dds");}
            if let Some(asset)=resolve(roots,&relative){index.missing.remove(n.get("id"));index.regions.insert(n.get("id").into(),Region{path:asset,rect:[n.num("x"),n.num("y"),n.num("width"),n.num("height")],descriptor:path.clone()});}
            else{index.regions.remove(n.get("id"));index.missing.insert(n.get("id").into(),(relative,path.clone()));}
        }
    } index
}

pub fn decode(path:&Path)->Result<egui::ColorImage,String>{
    let bytes=std::fs::read(path).map_err(|e|e.to_string())?;
    let rgba=if bytes.starts_with(b"DDS ") && bytes.len()>=128 && le(&bytes,80)?&4==0 {
        decode_raw_dds(&bytes)?
    }else if bytes.starts_with(b"DDS ") {
        let dds=image_dds::ddsfile::Dds::read(&mut std::io::Cursor::new(&bytes)).map_err(|e|format!("DDS header: {e}"))?;
        if dds.get_width()>16384||dds.get_height()>16384||dds.get_width() as u64*dds.get_height() as u64>67108864{return Err("DDS exceeds preview size limit".into())}
        image_dds::image_from_dds(&dds,0).map_err(|e|format!("DDS {:?}: {e}",image_dds::dds_image_format(&dds)))?
    }else{image::load_from_memory(&bytes).map_err(|e|e.to_string())?.into_rgba8()};
    Ok(egui::ColorImage::from_rgba_unmultiplied([rgba.width() as usize,rgba.height() as usize],rgba.as_raw()))
}
fn le(b:&[u8],offset:usize)->Result<u32,String>{Ok(u32::from_le_bytes(b.get(offset..offset+4).ok_or("Truncated DDS header")?.try_into().unwrap()))}
fn channel(value:u32,mask:u32,default:u8)->u8{if mask==0{return default}let shift=mask.trailing_zeros();let max=mask>>shift;(((value&mask)>>shift)as u64*255/max as u64)as u8}
fn decode_raw_dds(b:&[u8])->Result<image::RgbaImage,String>{
    let width=le(b,16)?;let height=le(b,12)?;let bits=le(b,88)?;let flags=le(b,80)?;
    if width==0||height==0||width>16384||height>16384||width as u64*height as u64>67108864{return Err("Unsupported DDS dimensions".into())}
    if ![8,16,24,32].contains(&bits){return Err(format!("Unsupported uncompressed DDS bit depth: {bits}"))}
    if flags&(0x40|0x20000|0x2)==0{return Err(format!("Unsupported DDS pixel flags: {flags:#x}"))}
    let masks=[le(b,92)?,le(b,96)?,le(b,100)?,le(b,104)?];
    let stride=(bits/8)as usize;let row=width as usize*stride;
    let pitch=if le(b,8)?&8!=0{(le(b,20)?as usize).max(row)}else{row};
    let needed=128usize.checked_add(pitch.checked_mul(height as usize).ok_or("DDS size overflow")?).ok_or("DDS size overflow")?;
    if b.len()<needed{return Err("Truncated DDS pixel data".into())}
    let mut output=image::RgbaImage::new(width,height);
    for y in 0..height{for x in 0..width{let pos=128+y as usize*pitch+x as usize*stride;let mut pixel=[0;4];pixel[..stride].copy_from_slice(&b[pos..pos+stride]);let v=u32::from_le_bytes(pixel);
        let rgba=if flags&0x20000!=0{let l=channel(v,masks[0],v as u8);[l,l,l,channel(v,masks[3],255)]}
        else if flags&0x40==0&&flags&0x2!=0{[255,255,255,channel(v,if masks[3]==0{255}else{masks[3]},255)]}
        else{[channel(v,masks[0],0),channel(v,masks[1],0),channel(v,masks[2],0),channel(v,masks[3],255)]};output.put_pixel(x,y,image::Rgba(rgba));
    }}Ok(output)
}

#[cfg(test)]mod tests{
    use super::*;
    fn header(bits:u32,flags:u32,masks:[u32;4],pitch:u32)->Vec<u8>{let mut b=vec![0;128];b[..4].copy_from_slice(b"DDS ");for(o,v)in [(4,124),(8,8),(12,1),(16,1),(20,pitch),(76,32),(80,flags),(88,bits),(92,masks[0]),(96,masks[1]),(100,masks[2]),(104,masks[3])]{b[o..o+4].copy_from_slice(&v.to_le_bytes());}b}
    #[test]fn bgra_with_alpha(){let mut b=header(32,0x41,[0xff0000,0xff00,0xff,0xff000000],4);b.extend([30,20,10,128]);assert_eq!(decode_raw_dds(&b).unwrap().get_pixel(0,0).0,[10,20,30,128]);}
    #[test]fn luminance_and_alpha_only(){let mut b=header(8,0x20000,[255,0,0,0],4);b.extend([90,0,0,0]);assert_eq!(decode_raw_dds(&b).unwrap().get_pixel(0,0).0,[90,90,90,255]);let mut a=header(8,2,[0,0,0,255],1);a.push(80);assert_eq!(decode_raw_dds(&a).unwrap().get_pixel(0,0).0,[255,255,255,80]);}
    #[test]fn truncated_pixels_rejected(){let b=header(32,0x40,[255,0,0,0],4);assert!(decode_raw_dds(&b).is_err());}
    #[test]fn bc1_through_bc7_decode(){
        let dir=std::env::temp_dir().join(format!("anomaly-bc-{}",std::process::id()));std::fs::create_dir_all(&dir).unwrap();
        for format in [71u32,74,77,80,83,95,98]{
            let mut b=header(0,4,[0;4],16);b[12..16].copy_from_slice(&4u32.to_le_bytes());b[16..20].copy_from_slice(&4u32.to_le_bytes());b[84..88].copy_from_slice(b"DX10");
            for v in [format,3,0,1,0]{b.extend_from_slice(&v.to_le_bytes());}b.extend(vec![0;if format==71||format==80{8}else{16}]);
            let path=dir.join(format!("{format}.dds"));std::fs::write(&path,b).unwrap();let im=decode(&path).unwrap_or_else(|e|panic!("DXGI {format}: {e}"));assert_eq!(im.size,[4,4]);
        }
    }
    #[test]fn layer_override_replaces_descriptor_and_image(){
        let dir=std::env::temp_dir().join(format!("anomaly-layers-{}",std::process::id()));let low=dir.join("base");let high=dir.join("mod");
        for r in [&low,&high]{std::fs::create_dir_all(r.join("configs/ui/textures_descr")).unwrap();std::fs::create_dir_all(r.join("textures/ui")).unwrap();std::fs::write(r.join("textures/ui/a.dds"),b"fixture").unwrap();}
        std::fs::write(low.join("configs/ui/textures_descr/a.xml"),"<w><file name='ui/a'><texture id='removed' width='1' height='1'/></file></w>").unwrap();
        std::fs::write(high.join("configs/ui/textures_descr/a.xml"),"<w><file name='ui/a'><texture id='new' width='2' height='3'/></file></w>").unwrap();
        let idx=build_index(&[low.clone(),high.clone()]);assert!(!idx.regions.contains_key("removed"));assert_eq!(idx.regions["new"].path,high.join("textures/ui/a.dds"));assert_eq!(idx.regions["new"].rect[2],2.0);
        let reversed=build_index(&[high,low.clone()]);assert!(reversed.regions.contains_key("removed"));assert_eq!(reversed.regions["removed"].path,low.join("textures/ui/a.dds"));
    }
}
