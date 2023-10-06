var searchIndex = JSON.parse('{\
"clausy":{"doc":"clausy transforms feature-model formulas into conjunctive …","t":"AAAAAAADLLLLLMLLLLLLLLLLMNNEDDGNNNRRENGLLLLLLLLLLLLLLLLLLLLLLLLLLLLLMMLOLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLLMLLLLLLLLLLLLLLLLLLLLLLLLLMLMMIIAAKKLFLAADLLMLLLLLLLLNDENRNLLLLNLLLNLLNLLLLLLNNNNNLFFLFLLLLLLLLNNNEDNRNLLLLLLLNNNLLNNLLLLLLNNNLFLFLNNLLLLLLLNNEDNRNLLLLLLLLLLNNLLLLLLLNNNLFLFMLLLLLLLLNFAAAAAAAFFFFF","n":["core","parser","shell","tests","util","clauses","formula","Clauses","assert_count","assert_valid","borrow","borrow_mut","clauses","clauses","count","count_featureide","fmt","from","from","into","to_string","try_from","try_into","type_id","vars","And","Aux","Expr","ExprInFormula","Formula","Id","Named","Not","Or","PRINT_ID","VAR_AUX_PREFIX","Var","Var","VarId","add_expr","add_var","add_var_aux","add_var_aux_expr","add_var_named","assert_canon","assert_valid","borrow","borrow","borrow","borrow","borrow_mut","borrow_mut","borrow_mut","borrow_mut","calc_hash","canon_visitor","children","clone","clone","clone_into","clone_into","def_and","def_or","eq","eq","equivalent","equivalent","expr","exprs","exprs_inv","flatten_expr","flatten_expr","fmt","fmt","fmt","fmt","fmt","fmt","format_expr","from","from","from","from","from","from","get_expr","get_root_expr","get_var_named","hash","hash","into","into","into","into","inval_expr","negate_exprs","new","nnf_visitor","parse","postorder_rev","preorder_rev","prepostorder_rev","reset_root_expr","root_id","set_expr","set_root_expr","simp_expr","sub_exprs","to_clauses","to_cnf_dist","to_cnf_tseitin","to_nnf","to_owned","to_owned","to_string","to_string","to_string","try_from","try_from","try_from","try_from","try_into","try_into","try_into","try_into","type_id","type_id","type_id","type_id","var_aux_id","var_expr","vars","vars_inv","FormulaParsee","FormulaParser","io","model","parse","parse_into","parse_new","parser","preprocess","sat","sat_inline","IoFormulaParser","borrow","borrow_mut","extension","from","into","new","parse_into","preprocess","try_from","try_into","type_id","EOI","ModelFormulaParser","Rule","WHITESPACE","_PEST_GRAMMAR_ModelFormulaParser","and","borrow","borrow","borrow_mut","borrow_mut","char","clone","clone_into","cmp","comment","eq","equivalent","expr","fmt","from","from","hash","into","into","line","name","not","or","paren_expr","parse","parse_children","parse_into","parse_into","parse_pair","partial_cmp","to_owned","try_from","try_from","try_into","try_into","type_id","type_id","unsupported","var","EOI","Rule","SatFormulaParser","WHITESPACE","_PEST_GRAMMAR_SatFormulaParser","and","borrow","borrow","borrow_mut","borrow_mut","clone","clone_into","cmp","comment","comment_text","comment_var","eq","equivalent","expr","file","fmt","from","from","hash","into","into","not","number","or","parse","parse_children","parse_into","parse_pair","partial_cmp","problem","space","to_owned","try_from","try_from","try_into","try_into","type_id","type_id","var","EOI","Rule","SatInlineFormulaParser","WHITESPACE","_PEST_GRAMMAR_SatInlineFormulaParser","and","borrow","borrow","borrow_mut","borrow_mut","can_parse","clone","clone_into","cmp","eq","equivalent","expr","file","fmt","from","from","hash","into","into","new","not","number","or","parse","parse_children","parse_into","parse_pair","parsed_ids","partial_cmp","to_owned","try_from","try_from","try_into","try_into","type_id","type_id","var","main","cnf","formula","parser","cnf_dist","nnf","valid","exec","file_exists","read_file","d4","io","path"],"q":[[0,"clausy"],[5,"clausy::core"],[7,"clausy::core::clauses"],[25,"clausy::core::formula"],[133,"clausy::parser"],[144,"clausy::parser::io"],[156,"clausy::parser::model"],[200,"clausy::parser::sat"],[244,"clausy::parser::sat_inline"],[286,"clausy::shell"],[287,"clausy::tests"],[290,"clausy::tests::formula"],[293,"clausy::util"],[296,"clausy::util::exec"]],"d":["Core data structures and algorithms on feature-model …","Parsers for feature-model formula files.","Imperative shell for operating on feature-model formulas.","Unit tests.","Miscellaneous utilities.","Clause representation of a feature-model formula.","Data structures and algorithms for feature-model formulas.","A Formula in its clause representation.","","Panics if this clause representation is invalid.","","","Returns the sub-expressions of a formula as clauses.","The clauses of this clause representation.","Counts the number of satisfying assignments of this clause …","","","Returns the argument unchanged.","","Calls <code>U::from(self)</code>.","","","","","The variables of this clause representation.","A conjunction of an expression.","An auxiliary variable.","An expression in a formula.","An expression that is explicitly paired with the formula …","A feature-model formula.","Identifier type for expressions.","A named variable.","A negation of an expression.","A disjunction of an expression.","Whether to print identifiers of expressions.","Prefix for auxiliary variables.","A variable in a formula.","A propositional variable.","Identifier type for variables.","Adds a new expression to this formula, returning its new …","Adds a new variable to this formula, returning its …","Adds a new auxiliary variable to this formula, returning …","Adds a new auxiliary variable to this formula, returning …","Adds a new named variable to this formula, returning its …","Panics if structural sharing is violated in this formula.","Panics if this formula is invalid.","","","","","","","","","Calculates the hash of this expression.","Transforms this formula into canonical form.","Returns the identifiers of the children of this expression.","","","","","Defines an And expression with a new auxiliary variable.","Defines an Or expression with a new auxiliary variable.","","","","","Adds or looks up an expression of this formula, returning …","Stores all expressions in this formula.","Maps expressions to their identifiers.","Flattens children of an expression into their parent.","Flattens children of an expression into their parent.","","","","","","","Writes an expression of this formula to a formatter.","Returns the argument unchanged.","Returns the argument unchanged.","Returns the argument unchanged.","Returns the argument unchanged.","","","Looks ups the identifier for an expression of this formula.","Returns the root expression of this formula.","Looks ups the identifier of a named variable in this …","","","Calls <code>U::from(self)</code>.","Calls <code>U::from(self)</code>.","Calls <code>U::from(self)</code>.","Calls <code>U::from(self)</code>.","Invalidates an expression after it was mutated.","Returns expressions that negate the given expressions.","Creates a new, empty formula.","Transforms this formula into negation normal form by …","","Visits all sub-expressions of this formula using a reverse …","Visits all sub-expressions of this formula using a reverse …","Visits all sub-expressions of this formula using a …","Resets the root expression, if necessary.","Specifies the root expression of this formula.","Mutates an expression in this formula.","Sets the root expression of this formula.","Simplifies an expression in this formula to an equivalent …","Manually enforces structural sharing in this formula.","","Transforms this formula into canonical conjunctive normal …","","Transforms this formula into canonical negation normal …","","","","","","","","","","","","","","","","","","Specifies the identifier of the most recently added …","Adds or looks up a named variable of this formula, …","Stores all variables in this formula.","Maps variables to their identifiers.","An object that can parse a feature-model formula file into …","Parses a feature-model formula file into a Formula …","Parser for any file format accepted by FeatureIDE.","Parser for KConfigReader .model files.","","Parses a feature-model formula file into an existing …","Parses a feature-model formula file into a new Formula.","Returns the appropriate parser for a file extension.","Preprocesses a feature-model formula file, if necessary.","Parser for DIMACS .sat files.","Parser for inline input in a .sat-like format.","Parses feature-model formula files in any file format …","","","The extension of the parsed file.","Returns the argument unchanged.","Calls <code>U::from(self)</code>.","","","","","","","","Parses feature-model formula files in the .model format.","","","","","","","","","","","","","","","","","","Returns the argument unchanged.","Returns the argument unchanged.","","Calls <code>U::from(self)</code>.","Calls <code>U::from(self)</code>.","","","","","","","","","","","","","","","","","","","","","","","Parses feature-model formula files in the .sat format.","","","","","","","","","","","","","","","","","","","Returns the argument unchanged.","Returns the argument unchanged.","","Calls <code>U::from(self)</code>.","Calls <code>U::from(self)</code>.","","","","","","","","","","","","","","","","","","","","","Parses inline input in a .sat-like format.","","","","","","","","","","","","","","","","","Returns the argument unchanged.","Returns the argument unchanged.","","Calls <code>U::from(self)</code>.","Calls <code>U::from(self)</code>.","","","","","","","","","","","","","","","","","","","Main entry point.","","","","","","","Utilities for executing external programs.","Returns whether a file exists at a given path.","Reads the contents and extension of a file.","Counts the number of satisfying assignments of some CNF in …","Converts a given feature-model file from one format into …","Returns the path of a bundled external program."],"i":[0,0,0,0,0,0,0,0,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,11,13,0,0,0,0,13,11,11,0,0,0,11,0,4,4,4,4,4,4,4,17,11,13,4,17,11,13,4,11,4,11,11,13,11,13,4,4,11,13,11,13,4,4,4,4,0,17,11,13,13,4,4,4,17,11,13,4,4,4,4,4,4,11,13,17,11,13,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,11,13,17,13,4,17,11,13,4,17,11,13,4,17,11,13,4,4,4,4,4,0,0,0,0,34,20,20,0,20,0,0,0,23,23,23,23,23,23,23,23,23,23,23,24,0,0,24,0,24,24,29,24,29,24,24,24,24,24,24,24,24,24,24,29,24,24,29,24,24,24,24,24,29,0,0,29,0,24,24,24,29,24,29,24,29,24,24,30,0,0,30,0,30,30,31,30,31,30,30,30,30,30,30,30,30,30,30,30,30,31,30,30,31,30,30,30,31,0,31,0,30,30,30,30,30,31,30,31,30,31,30,32,0,0,32,0,32,32,33,32,33,33,32,32,32,32,32,32,32,32,32,33,32,32,33,33,32,32,32,33,0,33,0,33,32,32,32,33,32,33,32,33,32,0,0,0,0,0,0,0,0,0,0,0,0,0],"f":[0,0,0,0,0,0,0,0,[[1,2,3]],[1],[[]],[[]],[4,[[6,[[6,[5]]]]]],0,[1,3],[[2,3],3],[[1,7],8],[[]],[4,1],[[]],[[],3],[[],9],[[],9],[[],10],0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,[[4,11],12],[[4,13],5],[4,5],[4,12],[[4,2],5],[4],[4,4],[[]],[[]],[[]],[[]],[[]],[[]],[[]],[[]],[11,14],[[4,12]],[11,[[15,[12]]]],[11,11],[13,13],[[]],[[]],[[4,12,[15,[12]]]],[[4,12,[15,[12]]]],[[11,11],16],[[13,13],16],[[],16],[[],16],[[4,11],12],0,0,[[4,11]],0,[[17,7],8],[[11,7],8],[[13,7],8],[[13,7],8],[[4,7],8],[[4,7],8],[[4,12,7],8],[[]],[[]],[[]],[[]],[[],4],[2,4],[[4,11],[[18,[12]]]],[4,12],[[4,2],[[18,[5]]]],[[11,19]],[[13,19]],[[]],[[]],[[]],[[]],[[4,12]],[[4,[6,[12]]],[[6,[12]]]],[[],4],[[4,12]],[[4,3,[21,[20]]],12],[[4,12,22]],[[4,12,22]],[[4,12,22,22]],[4],0,[[4,12,11]],[[4,12]],[[4,11]],[4,[[6,[12]]]],[4,1],[4,4],[4,4],[4,4],[[]],[[]],[[],3],[[],3],[[],3],[[],9],[[],9],[[],9],[[],9],[[],9],[[],9],[[],9],[[],9],[[],10],[[],10],[[],10],[[],10],0,[[4,2],12],0,0,0,0,0,0,[[3,[21,[20]]],12],[[3,4],12],[3,4],[[[18,[3]]],[[21,[20]]]],[3,3],0,0,0,[[]],[[]],0,[[]],[[]],[3,23],[[23,3,4],12],[[23,3],3],[[],9],[[],9],[[],10],0,0,0,0,0,0,[[]],[[]],[[]],[[]],0,[24,24],[[]],[[24,24],25],0,[[24,24],16],[[],16],0,[[24,7],8],[[]],[[]],[[24,19]],[[]],[[]],0,0,0,0,0,[[24,2],[[9,[[26,[24]],[27,[24]]]]]],[[[28,[24]],4],[[6,[12]]]],[[2,4],12],[[29,3,4],12],[[[28,[24]],4],12],[[24,24],[[18,[25]]]],[[]],[[],9],[[],9],[[],9],[[],9],[[],10],[[],10],0,0,0,0,0,0,0,0,[[]],[[]],[[]],[[]],[30,30],[[]],[[30,30],25],0,0,0,[[30,30],16],[[],16],0,0,[[30,7],8],[[]],[[]],[[30,19]],[[]],[[]],0,0,0,[[30,2],[[9,[[26,[30]],[27,[30]]]]]],[[[28,[30]],[15,[12]],4],[[6,[12]]]],[[31,3,4],12],[[[28,[30]],[15,[12]],4],12],[[30,30],[[18,[25]]]],0,0,[[]],[[],9],[[],9],[[],9],[[],9],[[],10],[[],10],0,0,0,0,0,0,0,[[]],[[]],[[]],[[]],[3,16],[32,32],[[]],[[32,32],25],[[32,32],16],[[],16],0,0,[[32,7],8],[[]],[[]],[[32,19]],[[]],[[]],[[[6,[12]]],33],0,0,0,[[32,2],[[9,[[26,[32]],[27,[32]]]]]],[[[28,[32]],[15,[12]],4],[[6,[12]]]],[[33,3,4],12],[[[28,[32]],[15,[12]],4],12],0,[[32,32],[[18,[25]]]],[[]],[[],9],[[],9],[[],9],[[],9],[[],10],[[],10],0,[[[6,[3]]]],0,0,0,0,0,0,0,[2,16],[2],[2,3],[[2,2,2],3],[2,3]],"c":[],"p":[[3,"Clauses"],[15,"str"],[3,"String"],[3,"Formula"],[15,"i32"],[3,"Vec"],[3,"Formatter"],[6,"Result"],[4,"Result"],[3,"TypeId"],[4,"Expr"],[15,"usize"],[4,"Var"],[15,"u64"],[15,"slice"],[15,"bool"],[3,"ExprInFormula"],[4,"Option"],[8,"Hasher"],[8,"FormulaParser"],[3,"Box"],[8,"FnMut"],[3,"IoFormulaParser"],[4,"Rule"],[4,"Ordering"],[3,"Pairs"],[3,"Error"],[3,"Pair"],[3,"ModelFormulaParser"],[4,"Rule"],[3,"SatFormulaParser"],[4,"Rule"],[3,"SatInlineFormulaParser"],[8,"FormulaParsee"]]}\
}');
if (typeof window !== 'undefined' && window.initSearch) {window.initSearch(searchIndex)};
if (typeof exports !== 'undefined') {exports.searchIndex = searchIndex};
