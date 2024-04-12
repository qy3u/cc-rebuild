use std::{
    fs,
    path::{Path, PathBuf},
    time::SystemTime,
};

use walkdir::WalkDir;

pub struct Build {
    inner: cc::Build,
    search: Vec<PathBuf>,
}

impl Build {
    pub fn new() -> Self {
        Build {
            inner: cc::Build::new(),
            search: Vec::new(),
        }
    }

    pub fn cuda(&mut self, cuda: bool) -> &mut Self {
        self.inner.cuda(cuda);
        self
    }

    pub fn cudart(&mut self, cudart: &str) -> &mut Self {
        self.inner.cudart(cudart);
        self
    }

    pub fn flag(&mut self, flag: &str) -> &mut Self {
        self.inner.flag(flag);
        self
    }

    pub fn include<P: AsRef<Path>>(&mut self, dir: P) -> &mut Self {
        self.search.push(dir.as_ref().to_owned());
        self.inner.include(dir);
        self
    }

    pub fn includes<P>(&mut self, dirs: P) -> &mut Build
    where
        P: IntoIterator,
        P::Item: AsRef<Path>,
    {
        let dirs: Vec<_> = dirs.into_iter().collect();
        for dir in dirs.iter() {
            self.search.push(dir.as_ref().to_owned());
        }

        self.inner.includes(dirs.into_iter());
        self
    }

    pub fn file<P: AsRef<Path>>(&mut self, p: P) -> &mut Self {
        self.inner.file(p);
        self
    }

    pub fn files<P>(&mut self, p: P) -> &mut Build
    where
        P: IntoIterator,
        P::Item: AsRef<Path>,
    {
        self.inner.files(p);
        self
    }

    pub fn compile(&self, output: &str) {
        if self.should_rebuild(output) {
            self.inner.compile(output);
        }
    }

    fn should_rebuild(&self, output: &str) -> bool {
        let lib_file = match find_static_file(output) {
            Some(lib) => lib,
            None => return true,
        };

        let since = lib_file
            .metadata()
            .expect("metadata")
            .modified()
            .expect("modified");

        let files = self.inner.get_files();

        let mut all_deps = Vec::new();
        for file in files {
            if !file.exists() {
                return true;
            }

            if changed_since(file, since) {
                return true;
            }

            match std::fs::read_to_string(file) {
                Ok(content) => {
                    let deps = crate::parse::cpp_dep(&content);
                    all_deps.extend(deps);
                }
                Err(_) => {
                    return true;
                }
            }
        }

        for dep in all_deps {
            if let Some(file) = find_include_file(&self.search, &dep) {
                if changed_since(file, since) {
                    return true;
                }
            }
        }

        false
    }
}

fn changed_since<P: AsRef<Path>>(file: P, since: SystemTime) -> bool {
    assert!(
        file.as_ref().exists(),
        "{} doesn't exists",
        file.as_ref().display()
    );

    let last_modify = fs::metadata(file)
        .expect("metadata")
        .modified()
        .expect("modified");

    last_modify > since
}

fn find_include_file(search: &[PathBuf], include: &str) -> Option<PathBuf> {
    for dir in search {
        let include = include.split("/");
        let mut file = dir.clone();
        for p in include {
            file = file.join(p);
        }

        if file.exists() {
            return Some(file);
        }
    }

    None
}

fn find_static_file(output: &str) -> Option<PathBuf> {
    let (_lib_name, gnu_lib_name) = if output.starts_with("lib") && output.ends_with(".a") {
        (&output[3..output.len() - 2], output.to_owned())
    } else {
        let mut gnu = String::with_capacity(5 + output.len());
        gnu.push_str("lib");
        gnu.push_str(output);
        gnu.push_str(".a");
        (output, gnu)
    };

    let lib_file = search_build(&gnu_lib_name);
    if lib_file.len() == 0 {
        return None;
    }

    if lib_file.len() >= 2 {
        panic!("more than one lib for {}", output);
    }

    Some(lib_file[1].clone().into_path())
}

fn search_build(name: &str) -> Vec<walkdir::DirEntry> {
    let root_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let build_dir = root_dir.join("target").join("release").join("build");
    WalkDir::new(build_dir)
        .into_iter()
        .filter_entry(|entry| {
            entry
                .file_name()
                .to_str()
                .map(|s| s == name)
                .unwrap_or(false)
        })
        .filter_map(Result::ok)
        .collect()
}
