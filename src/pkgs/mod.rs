use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs::read_dir;
use std::io::Write;
use std::path::Path;

use directories::ProjectDirs;
use libloading::{Library, Symbol};

use afed_objects::pkg::Pkg;

pub mod arr;
pub mod cls;
pub mod math;

const PACKAGE_FOLDERNAME: &str = "pkgs";
const PACKAGE_BUILDER: &str = "_build_pkg";

fn print_error<W: Write>(errout: &mut W, msg: String) {
    writeln!(errout, "Loading Error: {}\n", msg).expect("IO Error while writing loading error")
}

// Convert every member of a package into a global member
fn make_all_global(pkg: &mut Pkg) {
    if let Pkg::Map(map) = pkg {
        for (_, (is_global, _)) in map.iter_mut() {
            *is_global = true;
        }
    }
}

// Distinguish dynamic library files using file extensions
fn is_library(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }

    let ext = path.extension();
    if cfg!(windows) {
        ext.eq(&Some(OsStr::new("dll")))
    } else if cfg!(unix) {
        ext.eq(&Some(OsStr::new("so")))
    } else {
        true
    }
}

pub struct LoadedPkgs<'lib> {
    pkgs: HashMap<String, Pkg>,
    libs: &'lib mut Vec<Library>,
}

impl<'lib> LoadedPkgs<'lib> {
    pub fn new(libs: &'lib mut Vec<Library>) -> Self {
        LoadedPkgs {
            pkgs: HashMap::new(),
            libs,
        }
    }

    // Collect loaded packages into a single package
    pub fn into_pkg(self) -> Pkg {
        Pkg::Map(
            self.pkgs
                .into_iter()
                .map(|(name, pkg)| (name, (false, pkg)))
                .collect(),
        )
    }

    // Check for redefinition and add package
    fn add<W: Write>(&mut self, errout: &mut W, name: &str, pkg: Pkg) -> bool {
        if self.pkgs.contains_key(name) {
            print_error(
                errout,
                format!("Package name '{}' cannot be redefined", name),
            );
            return false;
        }
        self.pkgs.insert(name.into(), pkg);
        true
    }

    // Load package from file
    fn load<W: Write>(&mut self, errout: &mut W, libname: &Path) -> bool {
        let name: String;
        let pkg: Pkg;
        let lib: Library;

        if libname.as_os_str().is_empty() {
            print_error(errout, "Path to library must be non-empty".to_string());
            return false;
        }

        unsafe {
            match Library::new(libname) {
                Ok(result) => lib = result,
                Err(err) => {
                    print_error(errout, format!("Failed to load library, {}", err));
                    return false;
                }
            }

            let build: Symbol<unsafe extern "C" fn() -> (String, &'lib str, Pkg)> =
                match lib.get(PACKAGE_BUILDER.as_bytes()) {
                    Ok(result) => result,
                    Err(err) => {
                        print_error(
                            errout,
                            format!(
                                "Library should have symbol {:?}, {}\n",
                                PACKAGE_BUILDER, err
                            ),
                        );
                        return false;
                    }
                };

            let version: &str;
            (name, version, pkg) = (*build)();
            if version != afed_objects::VERSION {
                print_error(
                    errout,
                    format!(
                        "Library uses version \"{}\" of afed_objects, instead of version \"{}\"",
                        version,
                        afed_objects::VERSION
                    ),
                );
                return false;
            }
        }

        if self.add(errout, name.as_str(), pkg) {
            self.libs.push(lib);
            true
        } else {
            false
        }
    }

    // Load all the packages in `folder`
    pub fn load_from_folder<W: Write>(&mut self, errout: &mut W, folder: &Path) -> bool {
        let folder = match folder.canonicalize() {
            Ok(canonical) => canonical,
            Err(err) => {
                print_error(
                    errout,
                    format!("Cannot canonicalize '{}', {}", folder.display(), err),
                );
                return false;
            }
        };

        if let Ok(entry_iter) = read_dir(&folder) {
            for entry in entry_iter {
                match entry {
                    Ok(filename) => {
                        let buf = filename.path();
                        let path = buf.as_path();
                        if is_library(path) {
                            self.load(errout, path);
                        }
                    }
                    Err(err) => {
                        print_error(errout, format!("Cannot read directory entry, {}", err))
                    }
                }
            }
        } else {
            print_error(
                errout,
                format!("Failed to read entries of folder {}", folder.display()),
            );
            return false;
        }
        true
    }

    // Load all the packages in the default pkgs folder in the config folder
    pub fn load_from_config<W: Write>(&mut self, errout: &mut W) -> bool {
        let pkgs_folder = if let Some(folder) = ProjectDirs::from("", "", "Afed") {
            folder.config_dir().join(PACKAGE_FOLDERNAME)
        } else {
            print_error(errout, "Cannot find pkgs folder for afed".into());
            return false;
        };
        self.load_from_folder(errout, pkgs_folder.as_path())
    }

    pub fn build_bltns<W: Write>(&mut self, errout: &mut W) {
        let mut math = math::build_pkg();
        make_all_global(&mut math);
        self.add(errout, "math", math);

        let mut arr = arr::build_pkg();
        make_all_global(&mut arr);
        self.add(errout, "arr", arr);

        let mut cls = cls::build_pkg();
        make_all_global(&mut cls);
        self.add(errout, "cls", cls);
    }
}
