use serde::Serialize;

use common::{Literal, Operator, Prototype, Type};
pub use node::Node;

pub mod node;

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct Hir<T: VisitableNode> {
    functions: Vec<T>,
    structs: Vec<T>,
    prototypes: Vec<Prototype>,
}

impl<T: VisitableNode> Hir<T> {
    pub fn new() -> Self {
        Hir { functions: vec![], structs: vec![], prototypes: vec![] }
    }

    pub fn add_struct(&mut self, node: T) {
        self.structs.push(node)
    }

    pub fn add_function(&mut self, node: T) {
        self.functions.push(node);
    }

    pub fn add_prototype(&mut self, proto: Prototype) {
        self.prototypes.push(proto);
    }

    pub fn into_components(self) -> (Vec<T>, Vec<T>, Vec<Prototype>) {
        (self.structs, self.functions, self.prototypes)
    }

    pub fn functions(&self) -> &[T] {
        &self.functions
    }

    pub fn prototypes(&self) -> &[Prototype] {
        &self.prototypes
    }

    pub fn structs(&self) -> &[T] {
        &self.structs
    }
}

impl<T: VisitableNode> Default for Hir<T> {
    fn default() -> Self {
        Self::new()
    }
}

// Immutable visitor interface

pub trait Visitor {
    type AstNode;
    type Result;

    fn visit_node(&mut self, node: Self::AstNode) -> Self::Result;
    fn visit_for(
        &mut self, start_name: String, start_antn: Type, start_expr: Option<Node>, cond_expr: Node,
        step_expr: Node, body: Node,
    ) -> Self::Result;
    fn visit_let(&mut self, name: String, antn: Type, init: Option<Node>) -> Self::Result;
    fn visit_fn(&mut self, proto: Prototype, body: Option<Node>) -> Self::Result;
    fn visit_lit(&mut self, value: Literal<Node>, ty: Type) -> Self::Result;
    fn visit_ident(&mut self, name: String) -> Self::Result;
    fn visit_binop(&mut self, op: Operator, lhs: Node, rhs: Node) -> Self::Result;
    fn visit_unop(&mut self, op: Operator, rhs: Node) -> Self::Result;
    fn visit_call(&mut self, name: String, args: Vec<Node>) -> Self::Result;
    fn visit_cond(
        &mut self, cond_expr: Node, then_block: Node, else_block: Option<Node>, ty: Type,
    ) -> Self::Result;
    fn visit_block(&mut self, list: Vec<Node>) -> Self::Result;
    fn visit_index(&mut self, binding: Node, idx: Node) -> Self::Result;
    fn visit_fselector(&mut self, comp: Node, idx: u32) -> Self::Result;
}

pub trait VisitableNode {
    fn accept<V>(self, v: &mut V) -> V::Result
    where
        V: Visitor<AstNode = Self>;
}
