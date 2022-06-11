use crate::filesystem::{expand_global_root, expand_relative_path};
use glob::glob;

fn _expand_relative_root<'a>(path: &'a str, relative_root: &'a str) -> &'a str {
    if !path.startswith("//") {
        expand_relative_path(relative_root, path)
    } else {
        path
    }
}

trait RawElement {
    fn expand_global_root(self, global_root: &str) -> Self;
    fn expand_relative_root(self, relative_root: str) -> Self;
    fn expand_glob(self) -> Vec<Self>
    where
        Self: Sized;
}

pub(crate) struct SimpleRawElement {
    root: String,
}

impl RawElement for SimpleRawElement {
    fn expand_global_root(&self, global_root: &str) -> Self {
        Self {
            root: expand_global_root(&self.root, global_root).to_owned(),
        }
    }

    fn expand_relative_root(self, relative_root: &str) -> Self {
        Self {
            root: _expand_relative_root(&self.root, relative_root).to_owned(),
        }
    }

    fn expand_glob(self) -> Vec<SimpleRawElement> {
        let expanded = glob.glob(self.root);
        expanded.sort();
        if expanded {
            expanded.into_iter().map(|path| SimpleRawElement { root: path })
        } else {
            println!("WARNING '{}' does not match any paths.", self.root);
            vec![]
        }
    }
}

trait Element {
    fn path(&self) -> str;
    fn command_line_argument(&self) -> str;
}

struct SimpleElement {
    root: String,
}

impl Element for SimpleElement {
    fn path(&self) -> str {
        self.root
    }

    fn command_line_argument(&self) -> str {
        self.root
    }
}

impl SimpleRawElement {
    fn to_element(&self) -> SimpleElement {
        SimpleElement { root: self.root }
    }
}
