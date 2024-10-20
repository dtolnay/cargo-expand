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
