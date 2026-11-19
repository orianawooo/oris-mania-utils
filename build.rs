use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=assets/logo.ico");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    if env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("gnu") {
        let gcc = if Command::new("x86_64-w64-mingw32-gcc")
            .arg("--version")
            .output()
            .is_ok()
        {
            "x86_64-w64-mingw32-gcc"
        } else {
            "gcc"
        };
        if let Ok(out) = Command::new(gcc)
            .arg("-print-file-name=libstdc++.a")
            .output()
        {
            if out.status.success() {
                let raw = String::from_utf8_lossy(&out.stdout).trim().to_string();
                let lib_path = PathBuf::from(raw);
                if lib_path.is_file() {
                    if let Some(dir) = lib_path.parent() {
                        println!("cargo:rustc-link-search=native={}", dir.display());
                        println!("cargo:rustc-link-lib=static:+whole-archive=stdc++");
                    }
                }
            }
        }
    }

    let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") else {
        return;
    };
    let Ok(out_dir) = env::var("OUT_DIR") else {
        return;
    };

    let ico_path = PathBuf::from(&manifest_dir).join("assets").join("logo.ico");
    if !ico_path.is_file() {
        return;
    }

    let out_dir_pb = PathBuf::from(&out_dir);
    let rc_path = out_dir_pb.join("app_icon.rc");
    let obj_path = out_dir_pb.join("app_icon.o");
    let ico_escaped = ico_path.to_string_lossy().replace('\\', "/");
    let rc_content = format!("1 ICON \"{}\"\n", ico_escaped);

    if fs::write(&rc_path, rc_content).is_err() {
        return;
    }

    if env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        let res_path = out_dir_pb.join("app_icon.res");
        for rc_tool in ["rc.exe", "rc", "llvm-rc"] {
            if let Ok(status) = Command::new(rc_tool)
                .args([
                    "/nologo",
                    "/fo",
                    res_path.to_string_lossy().as_ref(),
                    rc_path.to_string_lossy().as_ref(),
                ])
                .status()
            {
                if status.success() && res_path.is_file() {
                    println!("cargo:rustc-link-arg-bins={}", res_path.display());
                    return;
                }
            }
        }
    }

    let tool = if Command::new("x86_64-w64-mingw32-windres")
        .arg("--version")
        .output()
        .is_ok()
    {
        "x86_64-w64-mingw32-windres"
    } else {
        "windres"
    };

    if let Ok(status) = Command::new(tool)
        .args([
            rc_path.to_string_lossy().as_ref(),
            "-O",
            "coff",
            "-o",
            obj_path.to_string_lossy().as_ref(),
        ])
        .status()
    {
        if status.success() && obj_path.is_file() {
            println!("cargo:rustc-link-arg-bins={}", obj_path.display());
        }
    }
}
