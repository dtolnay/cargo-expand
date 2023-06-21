use proc_macro2::{Ident, Span};
use quote::quote;
use std::panic;
use syn::fold::{self, Fold};
use syn::punctuated::Punctuated;
use syn::{
    token, Abi, Block, Expr, File, ForeignItem, Generics, ImplItem, Item, ItemConst, ItemFn,
    ItemForeignMod, ItemImpl, ItemTrait, ReturnType, Signature, Stmt, Token, TraitItem, Type,
    TypeInfer, Visibility,
};

pub(crate) fn unparse_maximal(syntax_tree: &File) -> String {
    if let Ok(formatted) = panic::catch_unwind(|| prettyplease::unparse(syntax_tree)) {
        return formatted;
    }

    let redacted = UnparseMaximal.fold_file(syntax_tree.clone());
    prettyplease::unparse(&redacted)
}

struct UnparseMaximal;

impl Fold for UnparseMaximal {
    fn fold_item(&mut self, item: Item) -> Item {
        let mut file = File {
            shebang: None,
            attrs: Vec::new(),
            items: vec![item],
        };

        let ok = panic::catch_unwind(|| prettyplease::unparse(&file)).is_ok();
        let item = file.items.pop().unwrap();
        if ok {
            return item;
        }

        file.items.push(fold::fold_item(self, item));
        let ok = panic::catch_unwind(|| prettyplease::unparse(&file)).is_ok();
        if ok {
            return file.items.pop().unwrap();
        }

        Item::Verbatim(quote!(...))
    }

    fn fold_stmt(&mut self, stmt: Stmt) -> Stmt {
        // `fn main() { $stmt }`
        let mut file = File {
            shebang: None,
            attrs: Vec::new(),
            items: vec![Item::Fn(ItemFn {
                attrs: Vec::new(),
                vis: Visibility::Inherited,
                sig: Signature {
                    constness: None,
                    asyncness: None,
                    unsafety: None,
                    abi: None,
                    fn_token: token::Fn(Span::call_site()),
                    ident: Ident::new("main", Span::call_site()),
                    generics: Generics::default(),
                    paren_token: token::Paren(Span::call_site()),
                    inputs: Punctuated::new(),
                    variadic: None,
                    output: ReturnType::Default,
                },
                block: Box::new(Block {
                    brace_token: token::Brace(Span::call_site()),
                    stmts: vec![stmt],
                }),
            })],
        };

        fn unwrap_item_fn(item: &mut Item) -> &mut ItemFn {
            match item {
                Item::Fn(item) => item,
                _ => unreachable!(),
            }
        }

        let ok = panic::catch_unwind(|| prettyplease::unparse(&file)).is_ok();
        let item_fn = unwrap_item_fn(&mut file.items[0]);
        let stmt = item_fn.block.stmts.pop().unwrap();
        if ok {
            return stmt;
        }

        item_fn.block.stmts.push(fold::fold_stmt(self, stmt));
        let ok = panic::catch_unwind(|| prettyplease::unparse(&file)).is_ok();
        if ok {
            let item_fn = unwrap_item_fn(&mut file.items[0]);
            return item_fn.block.stmts.pop().unwrap();
        }

        Stmt::Item(Item::Verbatim(quote!(...)))
    }

    fn fold_expr(&mut self, expr: Expr) -> Expr {
        // `const _: _ = $expr;`
        let mut file = File {
            shebang: None,
            attrs: Vec::new(),
            items: vec![Item::Const(ItemConst {
                attrs: Vec::new(),
                vis: Visibility::Inherited,
                const_token: Token![const](Span::call_site()),
                ident: Ident::from(Token![_](Span::call_site())),
                generics: Generics::default(),
                colon_token: Token![:](Span::call_site()),
                ty: Box::new(Type::Infer(TypeInfer {
                    underscore_token: Token![_](Span::call_site()),
                })),
                eq_token: Token![=](Span::call_site()),
                expr: Box::new(expr),
                semi_token: Token![;](Span::call_site()),
            })],
        };

        fn unwrap_item_const(item: Item) -> ItemConst {
            match item {
                Item::Const(item) => item,
                _ => unreachable!(),
            }
        }

        let ok = panic::catch_unwind(|| prettyplease::unparse(&file)).is_ok();
        let mut item_const = unwrap_item_const(file.items.pop().unwrap());
        let expr = *item_const.expr;
        if ok {
            return expr;
        }

        item_const.expr = Box::new(fold::fold_expr(self, expr));
        file.items.push(Item::Const(item_const));
        let ok = panic::catch_unwind(|| prettyplease::unparse(&file)).is_ok();
        if ok {
            let item_const = unwrap_item_const(file.items.pop().unwrap());
            return *item_const.expr;
        }

        Expr::Verbatim(quote!(...))
    }

    fn fold_foreign_item(&mut self, foreign_item: ForeignItem) -> ForeignItem {
        // `extern { $foreign_item }`
        let mut file = File {
            shebang: None,
            attrs: Vec::new(),
            items: vec![Item::ForeignMod(ItemForeignMod {
                attrs: Vec::new(),
                unsafety: None,
                abi: Abi {
                    extern_token: Token![extern](Span::call_site()),
                    name: None,
                },
                brace_token: token::Brace(Span::call_site()),
                items: vec![foreign_item],
            })],
        };

        fn unwrap_item_foreign_mod(item: &mut Item) -> &mut ItemForeignMod {
            match item {
                Item::ForeignMod(item) => item,
                _ => unreachable!(),
            }
        }

        let ok = panic::catch_unwind(|| prettyplease::unparse(&file)).is_ok();
        let item_foreign_mod = unwrap_item_foreign_mod(&mut file.items[0]);
        let foreign_item = item_foreign_mod.items.pop().unwrap();
        if ok {
            return foreign_item;
        }

        item_foreign_mod
            .items
            .push(fold::fold_foreign_item(self, foreign_item));
        let ok = panic::catch_unwind(|| prettyplease::unparse(&file)).is_ok();
        if ok {
            let item_foreign_mod = unwrap_item_foreign_mod(&mut file.items[0]);
            return item_foreign_mod.items.pop().unwrap();
        }

        ForeignItem::Verbatim(quote!(...))
    }

    fn fold_trait_item(&mut self, trait_item: TraitItem) -> TraitItem {
        // `trait Trait { $trait_item }`
        let mut file = File {
            shebang: None,
            attrs: Vec::new(),
            items: vec![Item::Trait(ItemTrait {
                attrs: Vec::new(),
                vis: Visibility::Inherited,
                unsafety: None,
                auto_token: None,
                restriction: None,
                trait_token: Token![trait](Span::call_site()),
                ident: Ident::new("Trait", Span::call_site()),
                generics: Generics::default(),
                colon_token: None,
                supertraits: Punctuated::new(),
                brace_token: token::Brace(Span::call_site()),
                items: vec![trait_item],
            })],
        };

        fn unwrap_item_trait(item: &mut Item) -> &mut ItemTrait {
            match item {
                Item::Trait(item) => item,
                _ => unreachable!(),
            }
        }

        let ok = panic::catch_unwind(|| prettyplease::unparse(&file)).is_ok();
        let item_trait = unwrap_item_trait(&mut file.items[0]);
        let trait_item = item_trait.items.pop().unwrap();
        if ok {
            return trait_item;
        }

        item_trait
            .items
            .push(fold::fold_trait_item(self, trait_item));
        let ok = panic::catch_unwind(|| prettyplease::unparse(&file)).is_ok();
        if ok {
            let item_trait = unwrap_item_trait(&mut file.items[0]);
            return item_trait.items.pop().unwrap();
        }

        TraitItem::Verbatim(quote!(...))
    }

    fn fold_impl_item(&mut self, impl_item: ImplItem) -> ImplItem {
        // `impl _ { $impl_item }`
        let mut file = File {
            shebang: None,
            attrs: Vec::new(),
            items: vec![Item::Impl(ItemImpl {
                attrs: Vec::new(),
                defaultness: None,
                unsafety: None,
                impl_token: Token![impl](Span::call_site()),
                generics: Generics::default(),
                trait_: None,
                self_ty: Box::new(Type::Infer(TypeInfer {
                    underscore_token: Token![_](Span::call_site()),
                })),
                brace_token: token::Brace(Span::call_site()),
                items: vec![impl_item],
            })],
        };

        fn unwrap_item_impl(item: &mut Item) -> &mut ItemImpl {
            match item {
                Item::Impl(item) => item,
                _ => unreachable!(),
            }
        }

        let ok = panic::catch_unwind(|| prettyplease::unparse(&file)).is_ok();
        let item_impl = unwrap_item_impl(&mut file.items[0]);
        let impl_item = item_impl.items.pop().unwrap();
        if ok {
            return impl_item;
        }

        item_impl.items.push(fold::fold_impl_item(self, impl_item));
        let ok = panic::catch_unwind(|| prettyplease::unparse(&file)).is_ok();
        if ok {
            let item_impl = unwrap_item_impl(&mut file.items[0]);
            return item_impl.items.pop().unwrap();
        }

        ImplItem::Verbatim(quote!(...))
    }
}
