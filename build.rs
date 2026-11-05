use std::{env,path::PathBuf,process::Command};
fn main(){
    println!("cargo:rerun-if-changed=assets/app.ico");
    println!("cargo:rerun-if-changed=assets/app.rc");
    let target=env::var("TARGET").unwrap();
    if !target.contains("windows"){return}
    let output=PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let resource=if target.ends_with("gnu"){
        let object=output.join("app-icon.o");
        let compiler=env::var("WINDRES").unwrap_or_else(|_|"windres".into());
        let status=Command::new(compiler).args(["-i","assets/app.rc","-O","coff","-o"]).arg(&object).status().expect("Windows resource compiler (windres) is required");
        assert!(status.success(),"Icon resource compilation failed");object
    }else{
        let object=output.join("app-icon.res");
        let status=Command::new("rc.exe").arg("/nologo").arg(format!("/fo{}",object.display())).arg("assets/app.rc").status().expect("Windows SDK resource compiler (rc.exe) is required");
        assert!(status.success(),"Icon resource compilation failed");object
    };
    println!("cargo:rustc-link-arg={}",resource.display());
}
