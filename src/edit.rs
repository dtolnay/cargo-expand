use syn::visit_mut::{self, VisitMut};
use syn::{
    Attribute, Block, Expr, ExprArray, ExprAssign, ExprAssignOp, ExprAsync, ExprAwait, ExprBinary,
    ExprBlock, ExprBox, ExprBreak, ExprCall, ExprCast, ExprClosure, ExprContinue, ExprField,
    ExprForLoop, ExprGroup, ExprIf, ExprIndex, ExprLet, ExprLit, ExprLoop, ExprMacro, ExprMatch,
    ExprMethodCall, ExprParen, ExprPath, ExprRange, ExprReference, ExprRepeat, ExprReturn,
    ExprStruct, ExprTry, ExprTryBlock, ExprTuple, ExprType, ExprUnary, ExprUnsafe, ExprWhile,
    ExprYield, File, Item, ItemMod, Stmt,
};

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

    fn visit_stmt_mut(&mut self, i: &mut Stmt) {
        match i {
            Stmt::Local(local) => remove_doc_attributes(&mut local.attrs),
            Stmt::Expr(e) | Stmt::Semi(e, _) => {
                if let Some(attrs) = attrs_mut(e) {
                    remove_doc_attributes(attrs);
                }
            }
            Stmt::Item(_) => {}
        }
        visit_mut::visit_stmt_mut(self, i);
    }
}

fn remove_macro_rules_from_vec_item(items: &mut Vec<Item>) {
    items.retain(|item| match item {
        Item::Macro(_) => false,
        _ => true,
    });
}

fn remove_doc_attributes(attrs: &mut Vec<Attribute>) {
    attrs.retain(|attr| !attr.path.is_ident("doc"));
}

fn attrs_mut(e: &mut Expr) -> Option<&mut Vec<Attribute>> {
    match e {
        Expr::Array(ExprArray { attrs, .. })
        | Expr::Assign(ExprAssign { attrs, .. })
        | Expr::AssignOp(ExprAssignOp { attrs, .. })
        | Expr::Async(ExprAsync { attrs, .. })
        | Expr::Await(ExprAwait { attrs, .. })
        | Expr::Binary(ExprBinary { attrs, .. })
        | Expr::Block(ExprBlock { attrs, .. })
        | Expr::Box(ExprBox { attrs, .. })
        | Expr::Break(ExprBreak { attrs, .. })
        | Expr::Call(ExprCall { attrs, .. })
        | Expr::Cast(ExprCast { attrs, .. })
        | Expr::Closure(ExprClosure { attrs, .. })
        | Expr::Continue(ExprContinue { attrs, .. })
        | Expr::Field(ExprField { attrs, .. })
        | Expr::ForLoop(ExprForLoop { attrs, .. })
        | Expr::Group(ExprGroup { attrs, .. })
        | Expr::If(ExprIf { attrs, .. })
        | Expr::Index(ExprIndex { attrs, .. })
        | Expr::Let(ExprLet { attrs, .. })
        | Expr::Lit(ExprLit { attrs, .. })
        | Expr::Loop(ExprLoop { attrs, .. })
        | Expr::Macro(ExprMacro { attrs, .. })
        | Expr::Match(ExprMatch { attrs, .. })
        | Expr::MethodCall(ExprMethodCall { attrs, .. })
        | Expr::Paren(ExprParen { attrs, .. })
        | Expr::Path(ExprPath { attrs, .. })
        | Expr::Range(ExprRange { attrs, .. })
        | Expr::Reference(ExprReference { attrs, .. })
        | Expr::Repeat(ExprRepeat { attrs, .. })
        | Expr::Return(ExprReturn { attrs, .. })
        | Expr::Struct(ExprStruct { attrs, .. })
        | Expr::Try(ExprTry { attrs, .. })
        | Expr::TryBlock(ExprTryBlock { attrs, .. })
        | Expr::Tuple(ExprTuple { attrs, .. })
        | Expr::Type(ExprType { attrs, .. })
        | Expr::Unary(ExprUnary { attrs, .. })
        | Expr::Unsafe(ExprUnsafe { attrs, .. })
        | Expr::While(ExprWhile { attrs, .. })
        | Expr::Yield(ExprYield { attrs, .. }) => Some(attrs),

        Expr::Verbatim(_) => None,

        #[cfg_attr(all(test, exhaustive), deny(non_exhaustive_omitted_patterns))]
        _ => None,
    }
}
