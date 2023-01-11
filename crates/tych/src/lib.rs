use common::{Literal, Operator, Prototype, Symbol, SymbolTable, Type};
use parse::ast::{self, Ast, VisitableNode, Visitor};

#[macro_use]
extern crate common;

mod macros;
#[cfg(test)]
mod tests;

// Performs the following tasks:
// - applies types to all nodes
// - checks for annotation consistency
// - checks for type consistency and relevance in binops
// - checks for type consistency in for step
// - checks for type consistency in if branches
// - checks main()'s annotation
// - checks for unknown functions, variables, and types
// - resolves function and struct names

pub struct Tych<'a> {
    symbol_table: &'a mut SymbolTable<Symbol>,
    types: Vec<String>,
    hint: Option<Type>,
    current_struct: Option<String>,
    module: String,
}

impl<'a> Tych<'a> {
    pub fn new(module: &str, symbol_table: &'a mut SymbolTable<Symbol>) -> Self {
        // XXX: see resolve_type()
        let mut types = Type::dump_types();
        types.append(&mut symbol_table.types());
        Tych { module: module.to_owned(), symbol_table, types, hint: None, current_struct: None }
    }

    pub fn walk(mut self, ast: Ast<ast::Node>) -> Result<Ast<ast::Node>, String> {
        let mut typed_ast = Ast::new();
        for node in ast.into_nodes() {
            let typed_node = node.accept(&mut self)?;
            typed_ast.add(typed_node)
        }
        Ok(typed_ast)
    }

    // Wrapper for `visit_node()` to handle hint updates
    fn check_node(&mut self, node: ast::Node, hint: Option<&Type>) -> Result<ast::Node, String> {
        self.hint = hint.cloned();
        self.visit_node(node)
    }

    fn check_lit_array(
        &mut self, lit: Literal<ast::Node>, ty_hint: Option<Type>,
    ) -> Result<(Literal<ast::Node>, Type), String> {
        // Extract the elements vec and the type of the array elements. Will always be None as
        // assigned by the parser as this point.
        let elements = match lit {
            Literal::Array { elements, .. } => elements,
            _ => unreachable!("expected array literal"),
        };

        // Clone the inner type hint
        // TODO: Could ty_hint be None?
        let (ty, size) = match ty_hint.unwrap() {
            Type::SArray(ty, sz) => (ty, sz),
            err => unreachable!("array literal has invalid type hint `{}`", err),
        };

        // Make sure array is big enough
        if elements.len() as u32 as usize > size {
            return Err(format!("SArray literal too big in assignment: `{}` > `{}`", elements.len(), size));
        }

        // Check every element and make sure they are uniform
        let mut chkd_elements = Vec::with_capacity(elements.len());
        for el in elements {
            let el_node = self.check_node(el, Some(&ty))?;
            let el_ty = el_node.ty().unwrap_or_default();
            if el_ty != ty.as_ref() {
                return Err(format!("Array literal's element wrong type: `{}` isn't a `{}`", el_node, ty));
            }
            chkd_elements.push(el_node);
        }

        // Rebuild the literal and return the type
        Ok((Literal::Array { elements: chkd_elements, inner_ty: Some(*ty.clone()) }, Type::SArray(ty, size)))
    }

    // Helper for variable initializations
    fn check_var_init(
        &mut self, name: &str, init: Option<&ast::Node>, antn: &Type, caller: &str,
    ) -> Result<Option<ast::Node>, String> {
        // If init exists, make sure it matches the variable's annotation
        if let Some(init) = init {
            let init_node = self.check_node(init.clone(), Some(antn))?;
            let init_ty = init_node.ty().unwrap_or_default();
            if antn != init_ty {
                return Err(format!(
                    "Types don't match in {}. `{}` annotated with `{}` but initial value is `{}`",
                    caller, name, antn, init_ty
                ));
            }
            Ok(Some(init_node))
        } else {
            Ok(None)
        }
    }

    // XXX: why check self.types for primitives?
    fn resolve_type(&self, ty: &Type) -> Option<Type> {
        if let Type::SArray(_, _) = ty {
            return Some(ty.to_owned());
        }

        let types = [ty.to_string(), format!("{}::{}", self.module, ty.to_string())];
        for ty_str in types {
            if self.types.contains(&ty_str) {
                if let Type::Comp(_) = ty {
                    return Some(Type::Comp(ty_str));
                }
                return Some(ty.to_owned());
            }
        }
        None
    }

    // Helper to get composite name and symbol for selector checking
    fn get_composite_symbol(&'a self, ty: Option<&'a Type>) -> Result<&'a Symbol, String> {
        let comp_name = match ty {
            Some(Type::Comp(name)) => name,
            Some(ty) => return Err(format!("Attempt to use selector on non-composite type: {}", ty)),
            None => unreachable!("no type for for selector target in tych"),
        };
        let comp_sym = self
            .symbol_table
            .resolve_symbol(&comp_name, &self.module)
            .ok_or(format!("Unknown composite type: `{}`", comp_name))?;
        Ok(comp_sym)
    }
}

impl<'a> ast::Visitor for Tych<'a> {
    type AstNode = ast::Node;
    type Result = Result<ast::Node, String>;

    fn visit_node(&mut self, node: Self::AstNode) -> Self::Result {
        node.accept(self)
    }

    fn visit_for(
        &mut self, start_name: String, start_antn: Type, start_expr: Option<ast::Node>, cond_expr: ast::Node,
        step_expr: ast::Node, body: ast::Node,
    ) -> Self::Result {
        // Insert starting variable
        self.symbol_table.enter_scope();
        self.symbol_table.insert(Symbol::new_var(&start_name, &start_antn, &self.module));

        let start_expr =
            self.check_var_init(&start_name, start_expr.as_ref(), &start_antn, "for statement")?;

        let start_antn = match self.resolve_type(&start_antn) {
            Some(ty) => ty,
            None => return Err(format!("Unknown type for start declaration in for loop: `{}`", start_antn)),
        };

        // Ensure the loop cond is always a bool
        let cond_expr = self.check_node(cond_expr, None)?;

        if cond_expr.ty().unwrap_or_default() != &Type::Bool {
            return Err("For loop conditional should always be a bool".to_string());
        }

        // Make sure the step type matches the starting variable
        let step_expr = self.check_node(step_expr, Some(&start_antn))?;
        let step_ty = step_expr.ty().unwrap_or_default();
        if step_ty != &start_antn {
            return Err(format!(
                "Step type mismatch in for statement. Step is `{}` but `{}` is `{}`",
                step_ty, start_name, start_antn
            ));
        }

        // Check body
        let body_node = self.check_node(body, None)?;

        self.symbol_table.leave_scope();

        Ok(ast::Node::new_for(start_name, start_antn, start_expr, cond_expr, step_expr, body_node))
    }

    fn visit_let(&mut self, name: String, antn: Type, init: Option<ast::Node>) -> Self::Result {
        let antn = match self.resolve_type(&antn) {
            Some(ty) => ty,
            None => return Err(format!("Unknown type in let declaration: `{}`", antn)),
        };

        // Don't process initization values for struct fields
        let init_node = if self.current_struct.is_none() {
            self.symbol_table.insert(Symbol::new_var(&name, &antn, &self.module));
            self.check_var_init(&name, init.as_ref(), &antn, "let statement")?
        } else if self.current_struct.is_some() && init.is_some() {
            return Err(format!("initializers aren't supported for struct fields at `{}`", name));
        } else {
            None
        };

        Ok(ast::Node::new_let(name, antn, init_node))
    }

    fn visit_fn(&mut self, proto: Prototype, body: Option<ast::Node>) -> Self::Result {
        let mut proto = proto;

        let fn_entry = match self.symbol_table.get(proto.name()).cloned() {
            Some(sym) => sym,
            None => unreachable!("missing symbol table entry for function: `{}`", proto.name()),
        };

        let ret_ty = match self.resolve_type(proto.ret_ty()) {
            Some(ty) => ty,
            None => {
                return Err(format!(
                    "Unknown return type in prototype for `{}`: `{}`",
                    proto.name(),
                    proto.ret_ty()
                ))
            },
        };

        // If body is None, this is an extern and no checking is needed
        let body = match body {
            Some(body) => body,
            None => return Ok(ast::Node::new_fn(proto, None)),
        };

        // Creates interstitial scope for the arguments in the function definition
        self.symbol_table.enter_scope();

        // Insert symbol for self if this is a method
        if let Some(name) = &self.current_struct {
            self.symbol_table.insert(Symbol::new_var("self", &Type::Comp(name.to_owned()), &self.module));
        }

        // Insert args into the local scope table
        let mut resolved_args = vec![];
        for arg in proto.args() {
            let arg_ty = match self.resolve_type(&arg.1) {
                Some(ty) => ty,
                None => {
                    return Err(format!(
                        "Unknown argument type in prototype `{}` for `{}`: `{}`",
                        proto.name(),
                        arg.0,
                        arg.1
                    ))
                },
            };
            self.symbol_table.insert(Symbol::new_var(&arg.0, &arg_ty, &self.module));
            resolved_args.push((arg.0.clone(), arg_ty));
        }
        proto.set_args(resolved_args);

        let body_node = self.check_node(body, None)?;
        let body_ty = body_node.ty().unwrap_or_default();

        // Make sure these are in sync since there's no `check_proto()`
        if proto.name() == "main" {
            if ret_ty != Type::Void {
                return Err(format!("main()'s return value shouldn't be annotated. Found `{}`", ret_ty));
            }
            proto.set_ret_ty(Type::Void);
        } else {
            proto.set_ret_ty(ret_ty.clone());
        }

        // Make sure function return type and the last statement match. Ignore
        // body type when proto is void.
        if &ret_ty != body_ty && ret_ty != Type::Void && proto.name() != "main" {
            return Err(format!(
                "Function `{}` should return type `{}` but last statement is `{}`",
                proto.name(),
                fn_entry.ret_ty(),
                body_ty
            ));
        }

        self.symbol_table.leave_scope();

        // XXX
        self.symbol_table.insert_with_name(proto.name(), Symbol::from(&proto));

        Ok(ast::Node::new_fn(proto, Some(body_node)))
    }

    // TODO: Check for circular struct definitions
    fn visit_struct(
        &mut self, name: String, fields: Vec<ast::Node>, methods: Vec<ast::Node>,
    ) -> Self::Result {
        if self.symbol_table.scope_depth() != 0 {
            return Err("structs can only be defined at the global level".to_string());
        }

        self.current_struct = Some(name.clone());
        let chkd_fields =
            fields.iter().map(|n| self.check_node(n.clone(), None)).collect::<Result<Vec<_>, String>>()?;
        let chkd_methods =
            methods.iter().map(|n| self.check_node(n.clone(), None)).collect::<Result<Vec<_>, String>>()?;
        self.current_struct = None;

        // Create a new symbol for the struct from the checked nodes. We do this to update
        // the symbol table with the fully resolved type names
        let mut sym_fields = vec![];
        for node in &chkd_fields {
            if let ast::Node { kind: ast::node::Kind::Let { name, antn, .. } } = node {
                sym_fields.push((name.to_owned(), antn.to_string()));
            }
        }
        let methods: Vec<_> = self
            .symbol_table
            .get(&name)
            .unwrap_or_else(|| unreachable!("missing symbol table entry for `{}` in `visit_struct()`", name))
            .methods()
            .unwrap_or_else(|| {
                unreachable!("missing struct symbol methods for `{}` in `visit_struct()`", name)
            })
            .into_iter()
            .map(|m| m.to_owned())
            .collect();
        self.symbol_table.insert(Symbol::new_struct(
            &name,
            Some(&sym_fields),
            Some(methods.as_slice()),
            &self.module,
            true,
        ));

        Ok(ast::Node::new_struct(name, chkd_fields, chkd_methods))
    }

    // If there's a type hint (in `self.hint`), use it or fail. If not, use the literal's
    // type. Update `lit` with the result and return the type.
    fn visit_lit(&mut self, value: Literal<ast::Node>, _ty: Option<Type>) -> Self::Result {
        use Literal::*;

        // TODO: Clean this up
        let lit = value;
        let (new_lit, lit_ty): (Literal<ast::Node>, Type) = match &self.hint {
            Some(hint) => match lit {
                Int8(v) => (Int8(v), Type::Int8),
                Int16(v) => (Int16(v), Type::Int16),
                Int32(v) => (Int32(v), Type::Int32),
                Int64(v) => (Int64(v), Type::Int64),
                UInt8(v) => (UInt8(v), Type::UInt8),
                UInt16(v) => (UInt16(v), Type::UInt16),
                UInt32(v) => (UInt32(v), Type::UInt32),
                UInt64(v) => match hint {
                    Type::Int8 => convert_num!(v, Int8, i8),
                    Type::Int16 => convert_num!(v, Int16, i16),
                    Type::Int32 => convert_num!(v, Int32, i32),
                    Type::Int64 => convert_num!(v, Int64, i64),
                    Type::UInt8 => convert_num!(v, UInt8, u8),
                    Type::UInt16 => convert_num!(v, UInt16, u16),
                    Type::UInt32 => convert_num!(v, UInt32, u32),
                    Type::UInt64 => convert_num!(v, UInt64, u64),
                    float_types!() => return Err("Literal is an integer in a float context".to_string()),
                    Type::Bool => return Err("Literal is an integer in a bool context".to_string()),
                    Type::Char => return Err("Literal is an integer in a char context".to_string()),
                    Type::SArray(..) => return Err("Literal is an integer in an sarray context".to_string()),
                    Type::Void => return Err("Literal is an integer in a void context".to_string()),
                    Type::Comp(_) => return Err("Literal is an integer in a compound context".to_string()),
                },
                Float(v) => match hint {
                    Type::Float => convert_num!(v, Float, f32),
                    Type::Double => convert_num!(v, Double, f64),
                    int_types!() => return Err("Literal is a float in an integer context".to_string()),
                    Type::Bool => return Err("Literal is a float in a bool context".to_string()),
                    Type::Char => return Err("Literal is a float in a char context".to_string()),
                    Type::SArray(..) => return Err("Literal is a float in an sarray context".to_string()),
                    _ => unreachable!("float conversion error"),
                },
                Double(v) => (Double(v), Type::Double),
                Bool(v) => (Bool(v), Type::Bool),
                Char(v) => (Char(v), Type::Char),
                Array { .. } => self.check_lit_array(lit, Some(hint.clone()))?,
                Comp(_) => unreachable!("composite types don't exist in the tych"),
            },
            None => match lit {
                Int32(v) => (Int32(v), Type::Int32), // Only used for main's return value
                UInt64(v) => {
                    let v = i32::try_from(v).map_err(|_| "Numeric literal out of range")?;
                    (Int32(v), Type::Int32)
                },
                Float(v) => (Float(v), Type::Float),
                Bool(v) => (Bool(v), Type::Bool),
                Char(v) => (Char(v), Type::Char),
                Array { .. } => self.check_lit_array(lit, None)?,
                x => unreachable!("numeric conversion error for {}", x),
            },
        };

        Ok(ast::Node::new_lit(new_lit, Some(lit_ty)))
    }

    fn visit_ident(&mut self, name: String, _ty: Option<Type>) -> Self::Result {
        let ident_ty =
            self.symbol_table.get(&name).ok_or(format!("Unknown variable: `{}`", name))?.ty().clone();
        Ok(ast::Node::new_ident(name, Some(ident_ty)))
    }

    // TODO: Check overflow on math ops
    fn visit_binop(
        &mut self, op: Operator, lhs: ast::Node, rhs: ast::Node, _ty: Option<Type>,
    ) -> Self::Result {
        use Operator::*;

        // Make sure LHS is a var in assignments
        if op == Assign
            && !matches!(
                lhs,
                ast::Node { kind: ast::node::Kind::Ident { .. } }
                    | ast::Node { kind: ast::node::Kind::Index { .. } }
                    | ast::Node { kind: ast::node::Kind::FSelector { .. } }
            )
        {
            return Err("Expected LHS to be a variable for assignment".to_string());
        }

        // Check if either side is a numeric literal. If so use the other side
        // as a type hint for the literal type.
        let (chkd_lhs, lhs_ty, chkd_rhs, rhs_ty);
        if lhs.is_num_literal() {
            chkd_rhs = self.check_node(rhs, None)?;
            rhs_ty = chkd_rhs.ty().unwrap_or_default();
            chkd_lhs = self.check_node(lhs, Some(rhs_ty))?;
            lhs_ty = chkd_lhs.ty().unwrap_or_default();
        } else {
            chkd_lhs = self.check_node(lhs, None)?;
            lhs_ty = chkd_lhs.ty().unwrap_or_default();
            chkd_rhs = self.check_node(rhs, Some(lhs_ty))?;
            rhs_ty = chkd_rhs.ty().unwrap_or_default();
        }

        // Both sides must match
        if lhs_ty != rhs_ty {
            return Err(format!("Mismatched types in binop: `{}` != `{}`", lhs_ty, rhs_ty));
        }

        // Check the operand types based on the operator used and set the
        // expression type accordingly
        let ty = match op {
            And | Or => {
                if lhs_ty != &Type::Bool || rhs_ty != &Type::Bool {
                    return Err(format!(
                        "Expected bools on either side of `{}`, got lhs: `{}`, rhs: `{}`",
                        op, lhs_ty, rhs_ty
                    ));
                }
                Type::Bool
            },
            Eq | NotEq => {
                match (&lhs_ty, &rhs_ty) {
                    (
                        numeric_types!() | Type::Bool | Type::Char,
                        numeric_types!() | Type::Bool | Type::Char,
                    ) => (),
                    _ => {
                        return Err(format!(
                            "Invalid type combination found in `{}` operation: (lhs: `{}`, rhs: `{}`)",
                            op, lhs_ty, rhs_ty
                        ))
                    },
                };
                Type::Bool
            },
            Gt | GtEq | Lt | LtEq => {
                match (&lhs_ty, &rhs_ty) {
                    (numeric_types!() | Type::Char, numeric_types!() | Type::Char) => (),
                    _ => {
                        return Err(format!(
                            "Invalid type combination found in `{}` operation: (lhs: `{}`, rhs: `{}`)",
                            op, lhs_ty, rhs_ty
                        ))
                    },
                };
                Type::Bool
            },
            Add | Div | Mul | Pow | Sub | BitAnd | BitXor | BitOr => {
                match (&lhs_ty, &rhs_ty) {
                    (numeric_types!(), numeric_types!()) => (),
                    _ => {
                        return Err(format!(
                            "Invalid type combination found in `{}` operation: (lhs: `{}`, rhs: `{}`)",
                            op, lhs_ty, rhs_ty
                        ))
                    },
                };
                lhs_ty.clone()
            },
            _ => Type::Void,
        };

        Ok(ast::Node::new_binop(op, chkd_lhs, chkd_rhs, Some(ty)))
    }

    fn visit_unop(&mut self, op: Operator, rhs: ast::Node, _ty: Option<Type>) -> Self::Result {
        let chkd_rhs = self.check_node(rhs, None)?;
        let rhs_ty = chkd_rhs.ty().cloned().unwrap_or_default();
        match rhs_ty {
            numeric_types!() => (),
            _ => {
                return Err(format!(
                    "Expected numeric type in unary operation `{}`, got rhs: `{}`",
                    op, rhs_ty
                ))
            },
        }
        Ok(ast::Node::new_unop(op, chkd_rhs, Some(rhs_ty)))
    }

    // TODO: errors in struct methods will display the partially lowered name. Pass in a
    // display name too.
    // XXX: I think we have this now with `fq_name`
    fn visit_call(&mut self, name: String, args: Vec<ast::Node>, _ty: Option<Type>) -> Self::Result {
        // Pull the function for the call from the table
        let fn_entry = self
            .symbol_table
            .resolve_symbol(&name, &self.module)
            .ok_or(format!("Call to undefined function: `{}`", name))?
            .clone();

        // Now that we have the FQN, use it in the AST
        let name = fn_entry
            .fq_name()
            .unwrap_or_else(|| unreachable!("non-function symbol in `visit_call()`"))
            .to_owned();

        // Pull out the function arg types
        let fe_arg_tys = fn_entry.arg_tys().to_vec();

        // Check arg length
        let fe_args_len = fe_arg_tys.len();
        let args_len = args.len();
        if fe_arg_tys.len() != args.len() {
            return Err(format!(
                "Call to `{}()` takes {} args and {} were given",
                name, fe_args_len, args_len
            ));
        }

        // Resolve the call's return type.
        let ret_ty = match self.resolve_type(fn_entry.ret_ty()) {
            Some(ty) => ty,
            None => unreachable!("unknown return type in `visit_call()`"),
        };

        // Check all args and record their types. Use the function entry arg types as type
        // hints.
        let mut chkd_args = Vec::with_capacity(args_len);
        let mut arg_tys = Vec::with_capacity(args_len);
        for (idx, expr) in args.into_iter().enumerate() {
            let chkd_arg = self.check_node(expr, Some(fe_arg_tys[idx]))?;
            arg_tys.push((idx, chkd_arg.ty().unwrap_or_default().clone()));
            chkd_args.push(chkd_arg);
        }

        // Make sure the function args and the call args jive
        fe_arg_tys.iter().zip(arg_tys).try_for_each(|(fa_ty, (idx, ca_ty))| {
            // Resolve param type first
            let fp_ty = match self.resolve_type(&fa_ty) {
                Some(ty) => ty,
                None => unreachable!("bad arg type in `visit_call()`"),
            };
            if fp_ty != ca_ty {
                Err(format!(
                    "Type mismatch in arg {} of call to `{}()`: `{}` != `{}`",
                    idx + 1,
                    name,
                    fp_ty,
                    ca_ty
                ))
            } else {
                Ok(())
            }
        })?;

        Ok(ast::Node::new_call(name, chkd_args, Some(ret_ty)))
    }

    fn visit_cond(
        &mut self, cond_expr: ast::Node, then_block: ast::Node, else_block: Option<ast::Node>,
        _ty: Option<Type>,
    ) -> Self::Result {
        let chkd_cond = self.check_node(cond_expr, None)?;
        let cond_ty = chkd_cond.ty().unwrap_or_default();
        if cond_ty != &Type::Bool {
            return Err("Conditional should always be a bool".to_string());
        }

        let chkd_then = self.check_node(then_block, None)?;
        let then_ty = chkd_then.ty().cloned().unwrap_or_default();

        // Consequent and alternate must match if else exists
        let mut chkd_else = None;
        if let Some(else_block) = else_block {
            let chkd_node = self.check_node(else_block, Some(&then_ty))?;
            let else_ty = chkd_node.ty().cloned().unwrap_or_default();
            chkd_else = Some(chkd_node);
            if then_ty != else_ty {
                return Err(format!(
                    "Both arms of conditional must be the same type: `then` == `{}`; `else` == `{}`",
                    then_ty, else_ty
                ));
            }
        }

        Ok(ast::Node::new_cond(chkd_cond, chkd_then, chkd_else, Some(then_ty)))
    }

    // Check the block expressions. Ensures statements always eval to void.
    fn visit_block(&mut self, list: Vec<ast::Node>, _ty: Option<Type>) -> Self::Result {
        self.symbol_table.enter_scope();

        // The block type is set to the final node's type
        let mut chkd_list = Vec::with_capacity(list.len());
        let mut list_ty = Type::Void;
        for node in list {
            let chkd_node = self.check_node(node, None)?;
            list_ty = chkd_node.ty().unwrap_or_default().clone();
            chkd_list.push(chkd_node);
        }

        self.symbol_table.leave_scope();

        Ok(ast::Node::new_block(chkd_list, Some(list_ty)))
    }

    fn visit_index(&mut self, binding: ast::Node, idx: ast::Node, _ty: Option<Type>) -> Self::Result {
        let chkd_binding = self.check_node(binding, None)?;
        let binding_ty = match chkd_binding.ty().unwrap_or_default() {
            Type::SArray(t, _) => *t.clone(),
            t => return Err(format!("Can't index `{}`", t)),
        };
        // TODO: Coerce into int32
        let chkd_idx = self.check_node(idx, Some(&Type::Int32))?;
        let idx_ty = chkd_idx.ty().unwrap_or_default();
        if !matches!(idx_ty, int_types!()) {
            return Err(format!("Array index must be an `int`, found `{}`", idx_ty));
        } else if !matches!(idx_ty, Type::Int32) {
            return Err("Index must be an int32 (for now)".to_string());
        }

        Ok(ast::Node::new_index(chkd_binding, chkd_idx, Some(binding_ty)))
    }

    fn visit_fselector(&mut self, comp: ast::Node, field: String, _ty: Option<Type>) -> Self::Result {
        let chkd_comp = self.check_node(comp, None)?;
        let comp_sym = self.get_composite_symbol(chkd_comp.ty())?;
        let field_ty: Type = comp_sym
            .fields()
            .unwrap_or_default()
            .into_iter()
            .find(|f| f.0 == field)
            .ok_or(format!("composite `{}` has no field: `{}`", comp_sym.name, field))?
            .1
            .into();

        let field_ty = match self.resolve_type(&field_ty) {
            Some(ty) => ty,
            None => unreachable!("bad field selector type in `visit_fselector()`"),
        };

        Ok(ast::Node::new_fselector(chkd_comp, field, Some(field_ty)))
    }

    fn visit_mselector(
        &mut self, comp: ast::Node, method_name: String, args: Vec<ast::Node>, ty: Option<Type>,
    ) -> Self::Result {
        let chkd_comp = self.check_node(comp, None)?;
        let comp_sym = self.get_composite_symbol(chkd_comp.ty())?.clone();

        // Make sure the method exists
        if !comp_sym.methods().unwrap_or_default().contains(&method_name.as_str()) {
            return Err(format!("composite `{}` has no method: `{}`", comp_sym.name, method_name));
        }
        let cooked_method_name = format!("_{}_{}", comp_sym.name, method_name);

        let chkd_call = self.visit_call(cooked_method_name.clone(), args, ty).map_err(|e| {
            // Don't use the cooked name for errors
            e.replace(&cooked_method_name, &format!("{}.{}", comp_sym.name, method_name))
        })?;

        match chkd_call.kind {
            ast::node::Kind::Call { name, args, ty } => {
                Ok(ast::Node::new_mselector(chkd_comp, name, args, ty))
            },
            _ => unreachable!("unknown node kind in `visit_mselector()`"),
        }
    }
}
