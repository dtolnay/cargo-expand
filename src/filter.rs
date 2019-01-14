use std::fmt::{self, Display};
use std::mem;
use std::str::FromStr;

use proc_macro2::Span;
use syn::{File, Ident, Item, ItemConst, ItemFn, ItemMod, ItemType, TraitItem, Visibility};

#[derive(Debug)]
pub struct Filter {
    segments: Vec<Ident>,
}

impl FromStr for Filter {
    type Err = String;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let mut segments = Vec::new();
        for segment in input.split("::") {
            match syn::parse_str(segment) {
                Ok(ident) => segments.push(ident),
                Err(_) => return Err(format!("`{}` is not an identifier", segment)),
            }
        }
        if segments.is_empty() {
            return Err("empty path".to_owned());
        }
        Ok(Filter { segments })
    }
}

impl Display for Filter {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        for (i, segment) in self.segments.iter().enumerate() {
            if i > 0 {
                formatter.write_str("::")?;
            }
            Display::fmt(segment, formatter)?;
        }
        Ok(())
    }
}

pub fn filter(syntax_tree: &mut File, filter: Filter) {
    syntax_tree.shebang = None;
    syntax_tree.attrs.clear();

    let items = mem::replace(&mut syntax_tree.items, Vec::new());
    let root_mod = ItemMod {
        attrs: Vec::new(),
        vis: Visibility::Inherited,
        mod_token: Default::default(),
        ident: Ident::new("root", Span::call_site()),
        content: Some((Default::default(), items)),
        semi: None,
    };
    let mut items = vec![Item::Mod(root_mod)];

    for segment in filter.segments {
        items = items
            .into_iter()
            .flat_map(enter_item)
            .filter(|item| name_of_item(item) == Some(&segment))
            .collect();
    }

    if items.len() == 1 {
        items = match items.pop().unwrap() {
            Item::Mod(ItemMod {
                content: Some((_, nested)),
                ..
            }) => nested,
            other => vec![other],
        };
    }

    syntax_tree.items = items;
}

fn enter_item(item: Item) -> Vec<Item> {
    match item {
        Item::ExternCrate(_) => Vec::new(),
        Item::Use(_) => Vec::new(),
        Item::Static(_) => Vec::new(),
        Item::Const(_) => Vec::new(),
        Item::Fn(_) => Vec::new(),
        Item::Mod(item_mod) => match item_mod.content {
            Some((_, nested)) => nested,
            None => Vec::new(),
        },
        Item::ForeignMod(_) => Vec::new(),
        Item::Type(_) => Vec::new(),
        Item::Existential(_) => Vec::new(),
        Item::Struct(_) => Vec::new(),
        Item::Enum(_) => Vec::new(),
        Item::Union(_) => Vec::new(),
        Item::Trait(item_trait) => item_trait
            .items
            .into_iter()
            .filter_map(|trait_item| match trait_item {
                TraitItem::Const(item) => Some(Item::Const(ItemConst {
                    attrs: item.attrs,
                    vis: Visibility::Inherited,
                    const_token: item.const_token,
                    ident: item.ident,
                    colon_token: item.colon_token,
                    ty: Box::new(item.ty),
                    eq_token: item.default.as_ref()?.0,
                    expr: Box::new(item.default?.1),
                    semi_token: item.semi_token,
                })),
                TraitItem::Method(item) => Some(Item::Fn(ItemFn {
                    attrs: item.attrs,
                    vis: Visibility::Inherited,
                    constness: item.sig.constness,
                    unsafety: item.sig.unsafety,
                    asyncness: item.sig.asyncness,
                    abi: item.sig.abi,
                    ident: item.sig.ident,
                    decl: Box::new(item.sig.decl),
                    block: Box::new(item.default?),
                })),
                TraitItem::Type(item) => Some(Item::Type(ItemType {
                    attrs: item.attrs,
                    vis: Visibility::Inherited,
                    type_token: item.type_token,
                    ident: item.ident,
                    generics: item.generics,
                    eq_token: item.default.as_ref()?.0,
                    ty: Box::new(item.default?.1),
                    semi_token: item.semi_token,
                })),
                TraitItem::Macro(_) => None,
                TraitItem::Verbatim(_) => None,
            })
            .collect(),
        Item::TraitAlias(_) => Vec::new(),
        Item::Impl(_) => Vec::new(),
        Item::Macro(_) => Vec::new(),
        Item::Macro2(_) => Vec::new(),
        Item::Verbatim(_) => Vec::new(),
    }
}

fn name_of_item(item: &Item) -> Option<&Ident> {
    match item {
        Item::ExternCrate(item) => match &item.rename {
            Some((_, rename)) => Some(rename),
            None => Some(&item.ident),
        },
        Item::Use(_) => None,
        Item::Static(item) => Some(&item.ident),
        Item::Const(item) => Some(&item.ident),
        Item::Fn(item) => Some(&item.ident),
        Item::Mod(item) => Some(&item.ident),
        Item::ForeignMod(_) => None,
        Item::Type(item) => Some(&item.ident),
        Item::Existential(item) => Some(&item.ident),
        Item::Struct(item) => Some(&item.ident),
        Item::Enum(item) => Some(&item.ident),
        Item::Union(item) => Some(&item.ident),
        Item::Trait(item) => Some(&item.ident),
        Item::TraitAlias(item) => Some(&item.ident),
        Item::Impl(_) => None,
        Item::Macro(item) => item.ident.as_ref(),
        Item::Macro2(item) => Some(&item.ident),
        Item::Verbatim(_) => None,
    }
}
