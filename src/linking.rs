use directories::ProjectDirs;
use libloading::{Library, Symbol};
use std::collections::HashMap;
use std::fs::read_dir;
use std::io::{Error, Write};
use std::path::Path;

use afed_objects::pkg::Pkg;

const PACKAGE_FOLDERNAME: &str = "pkgs";
const PACKAGE_BUILDER: &[u8] = b"_build_pkg\0";

// Load package from file
fn load(libname: &Path) -> Result<(String, Pkg, Library), libloading::Error> {
    let name: String;
    let pkg: Pkg;
    let lib: Library;

    unsafe {
        lib = Library::new(libname)?;
        let build: Symbol<unsafe extern "C" fn() -> (String, Pkg)> = lib.get(PACKAGE_BUILDER)?;
        (name, pkg) = (*build)();
    }
    Ok((name, pkg, lib))
}

// Load all the packages in `pkgs` folder
pub fn load_from_pkgs<W: Write>(
    errout: &mut W,
    pkg_map: &mut HashMap<String, Pkg>,
    libs: &mut Vec<Library>,
) -> Result<(), Error> {
    let pkgs_folder = if let Some(folder) = ProjectDirs::from("", "", "Afed") {
        folder.config_dir().join(PACKAGE_FOLDERNAME)
    } else {
        writeln!(
            errout,
            "Loading Warning: Cannot find pkgs folder for afed\n"
        )?;
        return Ok(());
    };

    if let Ok(entry_iter) = read_dir(pkgs_folder) {
        for entry in entry_iter {
            match entry {
                Ok(filename) => match load(filename.path().as_path()) {
                    Ok((name, pkg, lib)) => {
                        libs.push(lib);
                        if pkg_map.contains_key(&name) {
                            writeln!(
                                errout,
                                "Loading Error: Package name '{}' cannot be redefined\n",
                                name
                            )?;
                        }
                        pkg_map.insert(name, pkg);
                    }
                    Err(err) => {
                        writeln!(errout, "Loading Error: Failed to load library, {}\n", err)?
                    }
                },
                Err(err) => writeln!(errout, "Loading Error: {}\n", err)?,
            }
        }
    } else {
        writeln!(
            errout,
            "Loading Error: Failed to read entries of pkgs folder\n"
        )?;
    }
    Ok(())
}
