use std::{
    collections::{hash_map::DefaultHasher, HashMap, HashSet},
    fmt,
    hash::{Hash, Hasher},
    slice,
};
use Expr::*;

pub(crate) type Id = usize;
pub(crate) type VarId = i32;

#[derive(Debug)]
pub struct Formula<'a> {
    aux_root_id: Id,
    next_var_id: VarId,
    exprs: Vec<Expr>,
    exprs_inv: HashMap<u64, Vec<Id>>,
    vars: HashMap<VarId, &'a str>,
    vars_inv: HashMap<&'a str, VarId>,
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub(crate) enum Expr {
    Var(VarId),
    Not(Id),
    And(Vec<Id>),
    Or(Vec<Id>),
}

struct ExprInFormula<'a>(&'a Formula<'a>, &'a Id);

impl<'a> Formula<'a> {
    pub(crate) fn new() -> Self {
        Self {
            aux_root_id: 0,
            next_var_id: 0,
            exprs: vec![],
            exprs_inv: HashMap::new(), // possibly use with_capacity to avoid re-allocations
            vars: HashMap::new(),
            vars_inv: HashMap::new(),
        }
    }

    fn assert_valid(&self) {
        assert!(
            self.aux_root_id > 0 && self.exprs.len() > 0 && self.next_var_id > 0,
            "formula is invalid"
        );
    }

    fn get_root_expr(&self) -> Id {
        self.assert_valid();
        if let And(ids) = &self.exprs[self.aux_root_id] {
            assert!(ids.len() == 1, "aux root has more than one child");
            ids[0]
        } else {
            panic!("formula is invalid")
        }
    }

    pub(crate) fn set_root_expr(&mut self, root_id: Id) {
        let aux_root_id = self.expr(And(vec![root_id]));
        self.aux_root_id = aux_root_id;
    }

    fn hash_expr(expr: &Expr) -> u64 {
        let mut hasher = DefaultHasher::new();
        expr.hash(&mut hasher);
        hasher.finish()
    }

    fn add_expr(&mut self, expr: Expr) -> Id {
        let id = self.exprs.len();
        let hash = Self::hash_expr(&expr);
        self.exprs.insert(id, expr);
        self.exprs_inv.entry(hash).or_default().push(id);
        id
    }

    fn get_expr(&self, expr: &Expr) -> Option<Id> {
        let ids = self.exprs_inv.get(&Self::hash_expr(expr))?;
        for id in ids {
            if self.exprs[*id] == *expr {
                return Some(*id);
            }
        }
        None
    }

    pub(crate) fn expr(&mut self, expr: Expr) -> Id {
        self.get_expr(&expr).unwrap_or_else(|| self.add_expr(expr))
    }

    fn add_var(&mut self, var: &'a str) -> Id {
        let id = self.next_var_id + 1;
        self.vars.insert(id, var);
        self.vars_inv.insert(var, id);
        self.next_var_id += 1;
        self.expr(Var(id))
    }

    fn get_var(&mut self, var: &str) -> Option<Id> {
        Some(self.expr(Var(*self.vars_inv.get(var)?)))
    }

    pub(crate) fn var(&mut self, var: &'a str) -> Id {
        self.get_var(var).unwrap_or_else(|| self.add_var(var))
    }

    fn get_child_exprs<'b>(&self, expr: &'b Expr) -> &'b [Id] {
        match expr {
            Var(_) => &[],
            Not(id) => slice::from_ref(id),
            And(ids) | Or(ids) => ids,
        }
    }

    fn set_child_exprs(&mut self, id: Id, mut new_ids: Vec<Id>) {
        let expr = &self.exprs[id];
        let old_hash = Self::hash_expr(expr);
        for id in new_ids.iter_mut() {
            let child_expr = &self.exprs[*id];
            let child_hash = Self::hash_expr(expr);
            let dup_ids = self.exprs_inv.get(&child_hash).unwrap();
            for dup_id in dup_ids {
                if self.exprs[*dup_id] == *child_expr {
                    *id = *dup_id;
                    break;
                }
            }
        }

        let expr = &mut self.exprs[id];
        match expr {
            Var(_) => &[],
            Not(id) => {
                *id = new_ids[0];
                slice::from_ref(id)
            }
            And(ids) | Or(ids) => {
                *ids = new_ids;
                ids
            }
        };
        
        let expr = &self.exprs[id];
        let new_hash = Self::hash_expr(&expr);
        // here, the children of id change, so id's hash changes, possibly to some expr we already have - creating a possible duplicate
        // maybe allow temporary violation of the invariant until parent is traversed (have a temporary Set of hashes that collide and check that regularly)
        if new_hash != old_hash { // important so if no children are changed, the order in exprs_inv does not change (order is relevant for get_expr and dup_ids)
            self.exprs_inv.entry(old_hash).or_default().retain(|id2| *id2 != id); // probably, here only the first matching element has to be removed https://stackoverflow.com/questions/26243025
            self.exprs_inv.entry(new_hash).or_default().push(id); // probably, we could only push here if no equal expr has already been pushed (does this interact weirdly when there are true hash collisions involved?)
            if self.exprs_inv.get(&old_hash).unwrap().is_empty() {
                self.exprs_inv.remove(&old_hash);
            }
        }
    }

    fn negate_exprs(&mut self, ids: Vec<Id>) -> Vec<Id> {
        ids.iter().map(|id| self.expr(Not(*id))).collect()
    }

    fn child_exprs_refl<'b>(&'b self, id: &'b Id) -> &'b [Id] {
        match &self.exprs[*id] {
            Var(_) | Not(_) => slice::from_ref(&id),
            And(ids) | Or(ids) => &ids,
        }
    }

    fn is_non_aux_and(&self, id: Id) -> bool {
        if let And(_) = self.exprs[id] {
            id != self.aux_root_id
        } else {
            false
        }
    }

    fn splice_or(&self, clause_id: Id, new_clause: &mut Vec<Id>) {
        // splice child or's
        if let Or(literal_ids) = &self.exprs[clause_id] {
            for literal_id in literal_ids {
                new_clause.push(*literal_id);
            }
        } else {
            new_clause.push(clause_id);
        }
    }

    fn dedup(mut vec: Vec<Id>) -> Vec<Id> {
        // (inefficient) deduplication for idempotency
        vec.sort();
        vec.dedup();
        vec
    }

    pub(crate) fn get_vars(&self) -> HashMap<VarId, &str> {
        self.vars.clone()
    }

    // requires CNF
    pub(crate) fn get_clauses(&self) -> Vec<Vec<VarId>> {
        let mut clauses = Vec::<Vec<VarId>>::new();

        let add_literal = |id, clause: &mut Vec<VarId>| match self.exprs[id] {
            Var(var_id) => clause.push(var_id),
            Not(child_id) => {
                if let Var(var_id) = self.exprs[child_id] {
                    clause.push(-var_id);
                } else {
                    panic!("expected Var below Not, got {}", ExprInFormula(self, &id));
                }
            }
            _ => panic!(
                "expected Var or Not literal, got {}",
                ExprInFormula(self, &id)
            ),
        };

        let mut add_clause = |child_ids: &[Id]| {
            let mut clause = Vec::<VarId>::new();
            for child_id in child_ids {
                add_literal(*child_id, &mut clause);
            }
            clauses.push(clause);
        };

        match &self.exprs[self.get_root_expr()] {
            Var(_) | Not(_) => add_clause(slice::from_ref(&self.get_root_expr())),
            Or(child_ids) => add_clause(child_ids),
            And(child_ids) => {
                for child_id in child_ids {
                    match &self.exprs[*child_id] {
                        Var(_) | Not(_) => add_clause(slice::from_ref(child_id)),
                        Or(child_ids) => add_clause(child_ids),
                        _ => panic!(
                            "expected Var, Not, or Or expression, got {}",
                            ExprInFormula(self, child_id)
                        ),
                    }
                }
            }
        }

        clauses
    }

    fn format_expr(&self, id: Id, f: &mut fmt::Formatter) -> fmt::Result {
        // rewrite with preorder traversal?
        let mut write_helper = |kind: &str, ids: &[Id]| {
            write!(f, "{kind}(")?;
            for (i, id) in ids.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                self.format_expr(*id, f)?;
            }
            write!(f, ")")
        };
        match &self.exprs[id] {
            Var(var_id) => write!(f, "{}", self.vars.get(var_id).unwrap()),
            Not(id) => write_helper("Not", slice::from_ref(id)),
            And(ids) => write_helper("And", ids),
            Or(ids) => write_helper("Or", ids),
        }
    }

    fn print_expr(&mut self, id: Id) {
        println!("{}", ExprInFormula(self, &id));
    }

    // adds new expressions without discarding the old ones if they get orphaned (use Rc?)
    // creates temporary vector (use RefCell?)
    // may destroy structural sharing of originally shared subformulas,
    // so might be beneficial to not run this before Tseitin
    // (this might largely influence negation-CNF reasoning);
    // so, also a polarity-based Plaisted-Greenbaum implementation is necessary
    fn to_nnf_expr(&mut self, id: Id) {
        let mut child_ids: Vec<Id> = self.get_child_exprs(&self.exprs[id]).to_vec();

        for child_id in child_ids.iter_mut() {
            let child = &self.exprs[*child_id];
            match child {
                Var(_) | And(_) | Or(_) => (),
                Not(child2_id) => {
                    let child2 = &self.exprs[*child2_id];
                    match child2 {
                        Var(_) => (),
                        Not(child3_id) => {
                            *child_id = *child3_id;
                        }
                        And(child_ids2) => {
                            let new_expr = Or(self.negate_exprs(child_ids2.clone()));
                            *child_id = self.expr(new_expr);
                        }
                        Or(child_ids2) => {
                            let new_expr = And(self.negate_exprs(child_ids2.clone()));
                            *child_id = self.expr(new_expr); // what if we created an and, but are ourselves an and? could merge here!
                        }
                    }
                }
            }
        }

        self.set_child_exprs(id, child_ids);
    }

    // assumes NNF
    fn to_cnf_expr_dist(&mut self, id: Id) -> () {
        // need the children two times on the stack here, could maybe be disabled, but then merging is more complicated
        let child_ids = self.get_child_exprs(&self.exprs[id]).to_vec();
        let mut new_child_ids = Vec::<Id>::new();

        for child_id in child_ids {
            let child = &self.exprs[child_id];
            match child {
                Var(_) | Not(_) => new_child_ids.push(child_id),
                And(cnf_ids) => {
                    if self.is_non_aux_and(id) || cnf_ids.len() == 1 {
                        new_child_ids.extend(cnf_ids.clone());
                        // new_child_ids.push(self.expr(And(cnf))); // unoptimized version
                    } else {
                        new_child_ids.push(child_id);
                    }
                }
                Or(cnf_ids) => {
                    let mut clauses = Vec::<Vec<Id>>::new();
                    for (i, cnf_id) in cnf_ids.iter().enumerate() {
                        let clause_ids = self.child_exprs_refl(cnf_id);
                        if i == 0 {
                            clauses.extend(
                                // possibly this can be done with a neutral element instead
                                clause_ids
                                    .iter()
                                    .map(|clause_id| {
                                        let mut new_clause = Vec::<Id>::new();
                                        self.splice_or(*clause_id, &mut new_clause);
                                        new_clause
                                    })
                                    .collect::<Vec<Vec<Id>>>(),
                            );
                        } else {
                            let mut new_clauses = Vec::<Vec<Id>>::new();
                            for clause in &clauses {
                                for clause_id in clause_ids {
                                    let mut new_clause = clause.clone();
                                    self.splice_or(*clause_id, &mut new_clause);
                                    new_clauses.push(new_clause);
                                }
                            }
                            clauses = new_clauses;
                        }
                    }
                    let mut new_cnf_ids = Vec::<Id>::new();
                    for mut clause in clauses {
                        clause = Self::dedup(clause); // idempotency
                                                      // unary or
                        if clause.len() > 1 {
                            new_cnf_ids.push(self.expr(Or(clause)));
                        } else {
                            new_cnf_ids.push(clause[0]);
                        }
                    }
                    if self.is_non_aux_and(id) || new_cnf_ids.len() == 1 {
                        // splice into parent and
                        new_child_ids.extend(new_cnf_ids);
                        // new_child_ids.push(self.expr(And(cnf))); // unoptimized version
                    } else {
                        new_child_ids.push(self.expr(And(new_cnf_ids)));
                    }
                }
            }
        }

        self.set_child_exprs(id, Self::dedup(new_child_ids));
    }

    fn assert_shared_expr(&mut self, id: Id) {
        assert_eq!(self.get_expr(&self.exprs[id]).unwrap(), id);
    }

    // both traversals assume idempotent visitors that only mutate their children with set_child_exprs.
    // this is needed to ensure structural sharing in set_child_exprs.
    fn reverse_preorder(&mut self, visitor: fn(&mut Self, Id) -> ()) {
        self.assert_valid();
        let mut remaining_ids = vec![self.aux_root_id];
        // presumably, the following set can get large for large formulas (some for postorder traversal).
        // maybe it can be compacted in some way. (bit matrix? pre-sized vec<bool> with false as default?)
        let mut visited_ids = HashSet::<Id>::new();
        while !remaining_ids.is_empty() {
            let id = remaining_ids.pop().unwrap();
            if !visited_ids.contains(&id) {
                visitor(self, id);
                remaining_ids.extend(self.get_child_exprs(&self.exprs[id]));
                visited_ids.insert(id);
            }
        }

        // duplicate with above
        let aux_root_expr = &self.exprs[self.aux_root_id];
        let aux_root_hash = Self::hash_expr(aux_root_expr);
        let dup_ids = self.exprs_inv.get(&aux_root_hash).unwrap();
        for dup_id in dup_ids {
            if self.exprs.get(*dup_id).unwrap() == aux_root_expr {
                self.aux_root_id = *dup_id;
                break;
            }
        }
    }

    fn reverse_postorder(&mut self, visitor: fn(&mut Self, Id) -> ()) {
        self.assert_valid();
        let mut remaining_ids = vec![self.aux_root_id];
        let mut seen_ids = HashSet::<Id>::new();
        let mut visited_ids = HashSet::<Id>::new();
        while !remaining_ids.is_empty() {
            let id = remaining_ids.last().unwrap();
            let child_ids = self.get_child_exprs(&self.exprs[*id]);
            if !child_ids.is_empty() && !seen_ids.contains(id) && !visited_ids.contains(id) {
                seen_ids.insert(*id);
                remaining_ids.extend(child_ids);
            } else {
                if !visited_ids.contains(&id) {
                    visitor(self, *id);
                    visited_ids.insert(*id);
                    seen_ids.remove(id);
                }
                remaining_ids.pop();
            }
        }

        // duplicate with above
        let aux_root_expr = &self.exprs[self.aux_root_id];
        let aux_root_hash = Self::hash_expr(aux_root_expr);
        let dup_ids = self.exprs_inv.get(&aux_root_hash).unwrap();
        for dup_id in dup_ids {
            if self.exprs.get(*dup_id).unwrap() == aux_root_expr {
                self.aux_root_id = *dup_id;
                break;
            }
        }
    }

    // combine pre- and postorder to a DFS that creates NNF on first and distributive CNF on last visit

    pub fn print_subexprs(&mut self) {
        self.reverse_postorder(|s, i| s.print_expr(i));
    }

    pub fn to_nnf(mut self) -> Self {
        self.reverse_preorder(Self::to_nnf_expr);
        self
    }

    pub fn to_cnf_dist(mut self) -> Self {
        self.reverse_postorder(Self::to_cnf_expr_dist);
        self
    }

    pub fn assert_shared(&mut self) {
        self.reverse_preorder(Self::assert_shared_expr);
    }
}

impl<'a> fmt::Display for ExprInFormula<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.assert_valid();
        self.0.format_expr(*self.1, f)
    }
}

impl<'a> fmt::Display for Formula<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        ExprInFormula(self, &self.get_root_expr()).fmt(f)
    }
}
