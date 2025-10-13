use syn::visit_mut::{self, VisitMut};
use syn::{Block, File, Item, ItemMod, Stmt};

pub fn sanitize(syntax_tree: &mut File) {
    remove_macro_rules_from_vec_item(&mut syntax_tree.items);
    Sanitize.visit_file_mut(syntax_tree);
}

// - Remove all macro_rules
// - Remove doc attributes on statements (dtolnay/cargo-expand#71)
struct Sanitize;

impl VisitMut for Sanitize {
    fn visit_item_mod_mut(&mut self, i: &mut ItemMod) {
        if let Some((_, items)) = &mut i.content {
            remove_macro_rules_from_vec_item(items);
        }
        visit_mut::visit_item_mod_mut(self, i);
    }

    fn visit_block_mut(&mut self, i: &mut Block) {
        i.stmts.retain(|stmt| match stmt {
            Stmt::Item(Item::Macro(_)) => false,
            _ => true,
        });
        visit_mut::visit_block_mut(self, i);
    }
}

fn remove_macro_rules_from_vec_item(items: &mut Vec<Item>) {
    items.retain(|item| match item {
        Item::Macro(_) => false,
        _ => true,
    });
}

// - Remove all impl items with an #[automatically_derived] attribute
pub fn skip_auto_derived(syntax_tree: &mut File) {
    skip_auto_derived_from_vec_item(&mut syntax_tree.items);
    SkipAutoDerived.visit_file_mut(syntax_tree);
}

struct SkipAutoDerived;

impl VisitMut for SkipAutoDerived {
    fn visit_item_mod_mut(&mut self, i: &mut syn::ItemMod) {
        if let Some((_, items)) = &mut i.content {
            skip_auto_derived_from_vec_item(items);
        }
        visit_mut::visit_item_mod_mut(self, i);
    }
}

fn skip_auto_derived_from_vec_item(items: &mut Vec<Item>) {
    items.retain(|item| {
        if let Item::Impl(item_impl) = item {
            for attr in &item_impl.attrs {
                if is_automatically_derived_attr(attr) {
                    return false;
                }
            }
        }
        true
    });
}

fn is_automatically_derived_attr(attr: &syn::Attribute) -> bool {
    if let syn::Meta::Path(syn::Path {
            leading_colon: None,
            ref segments,
        }) = attr.meta {
        if let Some(seg) = segments.first() {
            return seg.arguments.is_empty() && seg.ident == "automatically_derived";
        }
    }
    false
}
