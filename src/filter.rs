use std::fmt::{self, Display};
use std::str::FromStr;

use syn::File;

#[derive(Debug)]
pub struct Filter {
    path: String,
}

impl FromStr for Filter {
    type Err = syn_select::Error;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let syntax_tree = File {
            shebang: None,
            attrs: Vec::new(),
            items: Vec::new(),
        };
        syn_select::select(input, &syntax_tree)?;
        Ok(Filter { path: input.to_owned() })
    }
}

impl Display for Filter {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        Display::fmt(&self.path, formatter)
    }
}

pub fn filter(syntax_tree: &mut File, filter: &Filter) {
    let items = match syn_select::select(&filter.path, syntax_tree) {
        Ok(items) => items,
        Err(err) => panic!("{}", err),
    };

    syntax_tree.shebang = None;
    syntax_tree.attrs.clear();
    syntax_tree.items = items;
}
