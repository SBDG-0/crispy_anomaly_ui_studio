use std::{collections::BTreeMap, ops::Range, path::Path};
use xmlparser::{ElementEnd, Token, Tokenizer};

#[derive(Clone, Debug)]
pub struct Attr { pub value: String, pub span: Range<usize> }
#[derive(Clone, Debug)]
pub struct Node {
    pub name: String, pub parent: Option<usize>, pub attrs: BTreeMap<String, Attr>,
    pub text: String,
}
impl Node {
    pub fn get(&self, name: &str) -> &str { self.attrs.get(name).map(|a|a.value.as_str()).unwrap_or("") }
    pub fn num(&self, name: &str) -> f32 { self.get(name).parse().unwrap_or(0.0) }
    pub fn drawable(&self) -> bool { self.attrs.contains_key("width") && self.attrs.contains_key("height") && self.name != "texture" }
}
#[derive(Clone)]
pub struct Document { pub source: String, pub nodes: Vec<Node>, pub legacy: bool, pub bom: bool, pub includes: Vec<String> }
impl Document {
    pub fn read(path: &Path) -> Result<Self,String> {
        let bytes=std::fs::read(path).map_err(|e|e.to_string())?;
        let bom=bytes.starts_with(&[239,187,191]);
        let bytes=if bom { &bytes[3..] } else { &bytes[..] };
        let (source,legacy)=match std::str::from_utf8(bytes) { Ok(s)=>(s.to_owned(),false),Err(_)=>(encoding_rs::WINDOWS_1251.decode(bytes).0.into_owned(),true) };
        Self::parse(source,legacy,bom)
    }
    pub fn parse(source:String, legacy:bool,bom:bool)->Result<Self,String> {
        // Mask engine preprocessor directives without changing UTF-8 byte positions.
        let mut masked=source.as_bytes().to_vec(); let mut offset=0; let mut includes=Vec::new();
        for line in source.split_inclusive('\n') {
            if line.trim_start().starts_with("#include") {
                includes.push(line.trim().to_owned());
                for b in &mut masked[offset..offset+line.len()] { if *b!=b'\r' && *b!=b'\n' {*b=b' ';} }
            } offset+=line.len();
        }
        let masked=String::from_utf8(masked).map_err(|e|e.to_string())?;
        let mut nodes:Vec<Node>=Vec::new(); let mut stack=Vec::new(); let mut pending=None;
        for token in Tokenizer::from(masked.as_str()) {
            match token.map_err(|e|e.to_string())? {
                Token::ElementStart{local,..}=> { let i=nodes.len(); nodes.push(Node{name:local.as_str().into(),parent:stack.last().copied(),attrs:BTreeMap::new(),text:String::new()}); pending=Some(i); }
                Token::Attribute{local,value,..}=>if let Some(i)=pending {nodes[i].attrs.insert(local.as_str().into(),Attr{value:value.as_str().into(),span:value.range()});},
                Token::ElementEnd{end,..}=>match end { ElementEnd::Open=>if let Some(i)=pending.take(){stack.push(i)},ElementEnd::Empty=>pending=None,ElementEnd::Close(_,local)=> { let i=stack.pop().ok_or("Unexpected closing tag")?; if nodes[i].name!=local.as_str(){return Err("Mismatched closing tag".into())} } },
                Token::Text{text}=>if let Some(&i)=stack.last(){nodes[i].text.push_str(text.as_str());},
                _=>{}
            }
        }
        if !stack.is_empty(){return Err("Unclosed element".into())}
        Ok(Self{source,nodes,legacy,bom,includes})
    }
    pub fn texture(&self,i:usize)->Option<&Node> {
        self.nodes.iter().find(|n| n.parent==Some(i)&&n.name=="texture").or_else(||{
            let progress=self.nodes.iter().position(|n|n.parent==Some(i)&&n.name=="progress")?;
            self.nodes.iter().find(|n|n.parent==Some(progress)&&n.name=="texture")
        })
    }
    pub fn patched(&self, edits:&BTreeMap<(usize,String),String>)->String {
        let mut patches:Vec<_>=edits.iter().filter_map(|((i,k),v)| self.nodes.get(*i)?.attrs.get(k).map(|a|(a.span.clone(),v))).collect();
        patches.sort_by_key(|(r,_)|std::cmp::Reverse(r.start));
        let mut output=self.source.clone(); for (span,value) in patches {output.replace_range(span,value);} output
    }
    pub fn bytes(&self,source:&str)->Result<Vec<u8>,String> {
        let mut result=if self.legacy {let (s,_,errors)=encoding_rs::WINDOWS_1251.encode(source);if errors{return Err("Text cannot be saved in the original encoding".into())}s.into_owned()}else{source.as_bytes().to_vec()};
        if self.bom {result.splice(0..0,[239,187,191]);} Ok(result)
    }
}

#[cfg(test)] mod tests {
    use super::*;
    #[test] fn preserves_comments_duplicate_nodes_and_includes(){
        let source="<w>\r\n#include \"ui\\background.xml\"\r\n<!-- keep -->\r\n<a x='2' width=\"3\" height=\"4\"/><a x=\"8\"/></w>";
        let d=Document::parse(source.into(),false,false).unwrap();
        let mut e=BTreeMap::new();e.insert((1,"x".into()),"12.5".into());
        assert_eq!(d.patched(&e),source.replacen("x='2'","x='12.5'",1)); assert_eq!(d.includes.len(),1);
    }
    #[test] fn encoding_roundtrip(){let d=Document::parse("<w><!-- Привет --></w>".into(),true,false).unwrap();let b=d.bytes(&d.source).unwrap();assert_eq!(encoding_rs::WINDOWS_1251.decode(&b).0,d.source);}
    #[test] fn rejects_bad_nesting(){assert!(Document::parse("<w><a></w>".into(),false,false).is_err());}
    #[test] fn edited_geometry_reparses_and_preserves_other_attributes(){
        let source="<w><main x=\"12\" y=\"728\" width=\"138\" height=\"16\" note=\"a &gt; b\"><texture>atlas_id</texture></main></w>";
        let d=Document::parse(source.into(),false,false).unwrap();let mut edits=BTreeMap::new();
        edits.insert((1,"x".into()),"67".into());edits.insert((1,"height".into()),"22.5".into());
        let out=d.patched(&edits);let again=Document::parse(out,false,false).unwrap();assert_eq!(again.nodes[1].num("x"),67.0);assert_eq!(again.nodes[1].num("height"),22.5);assert_eq!(again.nodes[1].get("note"),"a &gt; b");assert_eq!(again.texture(1).unwrap().text,"atlas_id");
    }
}
