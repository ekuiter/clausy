var searchIndex = JSON.parse('{\
"clausy":{"doc":"clausy transforms feature-model formulas into conjunctive …","t":"AAAAAAAAAAADLLLLLLLLLLLLLLMMLOLLLLLLLLMMLLLLLLLOLLLMLLLMMDLLLLMLLLLLLLLLLLLLMNEGNNNLLLLLLLLLLLLLLLLDLLLLLLLLLMLMLLLLLLLLDMLLLMLLLLLLNNEGLLLLLLLLLLLLLLLLIIAAKKLFLAADLLMLLLLLLLLNDENRNLLLLNLLLNLLNLLLLLLNNNNNLFFLFLLLLLLLLNNNEDNRNLLLLLLLNNNLLNNLLLLLLNNNLFLFLNNLLLLLLLNNEDNRMNLLLLLLLLLLNNLMLLLLLLNNNLLLLLLLLLLLLNRROOFFFAAAAAAAFFFFFFF","n":["core","parser","shell","tests","util","arena","clauses","expr","formula","formula_ref","var","Arena","add_expr","add_var","add_var_aux","add_var_aux_expr","add_var_named","as_formula","borrow","borrow_mut","canon_visitor","cnf_dist_visitor","cnf_tseitin_visitor","def_and","def_or","expr","exprs","exprs_inv","flatten_expr","flatten_expr","format_expr","from","get_expr","get_var_named","into","inval_expr","negate_exprs","new","new_exprs","new_vars","nnf_visitor","parse","postorder_rev","preorder_rev","prepostorder_rev","set_expr","simp_expr","simp_expr","try_from","try_into","type_id","var_aux_id","var_expr","var_expr_with_id","vars","vars","vars_inv","Clauses","assert_count","borrow","borrow_mut","clauses","clauses","count","count_featureide","enumerate","fmt","from","from","into","satisfy","solution_to_string","to_string","try_from","try_into","type_id","vars","And","Expr","ExprId","Not","Or","Var","borrow","borrow_mut","calc_hash","children","clone","clone_into","eq","equivalent","fmt","from","hash","into","to_owned","try_from","try_into","type_id","Formula","as_ref","assert_canon","borrow","borrow_mut","from","from","into","new","reset_root_expr","root_id","sub_exprs","sub_var_ids","sub_vars","to_canon","to_cnf_dist","to_cnf_tseitin","to_nnf","try_from","try_into","type_id","FormulaRef","arena","borrow","borrow_mut","fmt","formula","from","into","to_string","try_from","try_into","type_id","Aux","Named","Var","VarId","borrow","borrow_mut","clone","clone_into","eq","equivalent","fmt","fmt","from","hash","into","to_owned","to_string","try_from","try_into","type_id","FormulaParsee","FormulaParser","io","model","parse","parse_into","parse_new","parser","preprocess","sat","sat_inline","IoFormulaParser","borrow","borrow_mut","extension","from","into","new","parse_into","preprocess","try_from","try_into","type_id","EOI","ModelFormulaParser","Rule","WHITESPACE","_PEST_GRAMMAR_ModelFormulaParser","and","borrow","borrow","borrow_mut","borrow_mut","char","clone","clone_into","cmp","comment","eq","equivalent","expr","fmt","from","from","hash","into","into","line","name","not","or","paren_expr","parse","parse_children","parse_into","parse_into","parse_pair","partial_cmp","to_owned","try_from","try_from","try_into","try_into","type_id","type_id","unsupported","var","EOI","Rule","SatFormulaParser","WHITESPACE","_PEST_GRAMMAR_SatFormulaParser","and","borrow","borrow","borrow_mut","borrow_mut","clone","clone_into","cmp","comment","comment_text","comment_var","eq","equivalent","expr","file","fmt","from","from","hash","into","into","not","number","or","parse","parse_children","parse_into","parse_pair","partial_cmp","problem","space","to_owned","try_from","try_from","try_into","try_into","type_id","type_id","var","EOI","Rule","SatInlineFormulaParser","WHITESPACE","_PEST_GRAMMAR_SatInlineFormulaParser","add_backbone_literals","and","borrow","borrow","borrow_mut","borrow_mut","can_parse","clone","clone_into","cmp","eq","equivalent","expr","file","fmt","formulas","from","from","hash","into","into","new","not","number","or","parse","parse_children","parse_into","parse_pair","partial_cmp","to_owned","try_from","try_from","try_into","try_into","type_id","type_id","var","PRINT_ID","VAR_AUX_PREFIX","clauses","formula","main","name_from_io","name_to_io","cnf","formula","parser","cnf_dist","nnf","valid","exec","file_exists","read_file","bc_minisat_all","d4","io","kissat","path"],"q":[[0,"clausy"],[5,"clausy::core"],[11,"clausy::core::arena"],[57,"clausy::core::clauses"],[77,"clausy::core::expr"],[99,"clausy::core::formula"],[120,"clausy::core::formula_ref"],[132,"clausy::core::var"],[152,"clausy::parser"],[163,"clausy::parser::io"],[175,"clausy::parser::model"],[219,"clausy::parser::sat"],[263,"clausy::parser::sat_inline"],[306,"clausy::shell"],[313,"clausy::tests"],[316,"clausy::tests::formula"],[319,"clausy::util"],[322,"clausy::util::exec"]],"d":["Core data structures and algorithms on feature-model …","Parsers for feature-model formula files.","Imperative shell for operating on feature-model formulas.","Unit tests.","Miscellaneous utilities.","Defines an arena of variables and expressions.","Clause representation of a feature-model formula.","Defines expressions in an arena.","Defines a feature-model formula.","Defines a reference to a feature-model formula.","Defines variables in an arena.","An arena of variables and expressions.","Adds a new expression to this arena, returning its new …","Adds a new variable to this arena, returning its …","Adds a new auxiliary variable to this arena, returning its …","Adds a new auxiliary variable to this arena, returning its …","Adds a new named variable to this arena, returning its …","Returns a formula with the given root expression.","","","Transforms an expression into canonical form (see …","Transforms an expression into canonical conjunctive normal …","Transforms an expression into canonical conjunctive normal …","Defines an And expression with a new auxiliary variable.","Defines an Or expression with a new auxiliary variable.","Adds or looks up an expression in this arena, returning …","Stores all expressions in this arena.","Maps expressions to their identifiers.","Flattens children of an expression into their parent.","Flattens children of an expression into their parent.","Writes an expression in this arena to a formatter.","Returns the argument unchanged.","Looks ups the identifier for an expression in this arena.","Looks ups the identifier of a named variable in this arena.","Calls <code>U::from(self)</code>.","Invalidates an expression after it was mutated.","Returns expressions that negate the given expressions.","Creates a new, empty arena.","Temporarily stores new expressions that are not yet …","Temporarily stores new variables created that are not yet …","Transforms an expression into negation normal form by …","","Visits all sub-expressions of a given root expression …","Visits all sub-expressions of a given root expression …","Visits all sub-expressions of a given root expression …","Mutates an expression in this arena.","Simplifies an expression in this arena to an equivalent …","Simplifies an expression in an arena to an equivalent one.","","","","Specifies the identifier of the most recently added …","Adds or looks up a named variable in this arena, returning …","Adds or looks up a named variable in this arena, returning …","Returns all variables and their identifiers in this arena …","Stores all variables in this arena.","Maps variables to their identifiers.","A super::formula::Formula in its clause representation.","Panics if this clause representation has a different model …","","","Returns the sub-expressions of a formula as clauses.","The clauses of this clause representation.","Counts the number of solutions of this clause …","Counts the number of solutions of a feature-model file …","Enumerates all solutions of this clause representation.","","","Returns the argument unchanged.","Calls <code>U::from(self)</code>.","Attempts to finds a solution of this clause representation.","Returns a solution as a human-readable string.","","","","","The variables of this clause representation.","A conjunction of an expression.","An expression in an arena.","Identifier type for expressions.","A negation of an expression.","A disjunction of an expression.","A propositional variable.","","","Calculates the hash of this expression.","Returns the identifiers of the children of this expression.","","","","","","Returns the argument unchanged.","","Calls <code>U::from(self)</code>.","","","","","A feature-model formula.","Returns a shared reference to this formula in the context …","Panics if structural sharing is violated in this formula.","","","","Returns the argument unchanged.","Calls <code>U::from(self)</code>.","Creates a new formula.","Resets the root expression of this formula, if necessary.","Specifies the root expression of this formula.","Returns the identifiers of all sub-expressions of this …","Specifies the sub-variables of this formula.","Returns all sub-variables of this formula and their …","Transforms this formula into canonical form (see …","Transforms this formula into canonical conjunctive normal …","Transforms this formula into canonical conjunctive normal …","Transforms this formula into canonical negation normal …","","","","A shared reference to a feature-model formula.","","","","","","Returns the argument unchanged.","Calls <code>U::from(self)</code>.","","","","","An auxiliary variable.","A named variable.","A variable in an arena.","Identifier type for variables.","","","","","","","","","Returns the argument unchanged.","","Calls <code>U::from(self)</code>.","","","","","","An object that can parse a feature-model formula file into …","Parses a feature-model formula file into an Arena.","Parser for any file format accepted by FeatureIDE.","Parser for KConfigReader .model files.","Parses a feature-model formula into this object.","Parses a feature-model formula file into an existing Arena.","Parses a feature-model formula file into a new Arena.","Returns the appropriate parser for a file extension.","Preprocesses a feature-model formula file, if necessary.","Parser for DIMACS .sat files.","Parser for inline input in a .sat-like format.","Parses feature-model formula files in any file format …","","","The extension of the parsed file.","Returns the argument unchanged.","Calls <code>U::from(self)</code>.","","","","","","","","Parses feature-model formula files in the .model format.","","","","","","","","","","","","","","","","","","Returns the argument unchanged.","Returns the argument unchanged.","","Calls <code>U::from(self)</code>.","Calls <code>U::from(self)</code>.","","","","","","","","","","","","","","","","","","","","","","","Parses feature-model formula files in the .sat format.","","","","","","","","","","","","","","","","","","","Returns the argument unchanged.","Returns the argument unchanged.","","Calls <code>U::from(self)</code>.","Calls <code>U::from(self)</code>.","","","","","","","","","","","","","","","","","","","","","Parses inline input in a .sat-like format.","","","","","","","","","","","","","","","","","","","Returns the argument unchanged.","Returns the argument unchanged.","","Calls <code>U::from(self)</code>.","Calls <code>U::from(self)</code>.","","","","","","","","","","","","","","","","","","Whether to print identifiers of expressions.","Prefix for auxiliary variables.","Converts a formula into its clause representation, if not …","","Main entry point.","","","","","","","","","Utilities for executing external programs.","Returns whether a file exists at a given path.","Reads the contents and extension of a file.","Enumerates all solutions of some CNF in DIMACS format.","Counts the number of solutions of some CNF in DIMACS …","Converts a given feature-model file from one format into …","Attempts to find a solution of some CNF in DIMACS format.","Returns the path of a bundled external program."],"i":[0,0,0,0,0,0,0,0,0,0,0,0,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,0,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,0,1,1,1,1,1,1,1,1,1,0,20,20,20,20,20,20,20,20,20,20,20,20,20,20,20,20,20,20,20,2,0,0,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,0,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,0,21,21,21,21,21,21,21,21,21,21,21,4,4,0,0,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,0,0,0,0,38,14,14,0,14,0,0,0,27,27,27,27,27,27,27,27,27,27,27,28,0,0,28,0,28,28,33,28,33,28,28,28,28,28,28,28,28,28,28,33,28,28,33,28,28,28,28,28,33,0,0,33,0,28,28,28,33,28,33,28,33,28,28,34,0,0,34,0,34,34,35,34,35,34,34,34,34,34,34,34,34,34,34,34,34,35,34,34,35,34,34,34,35,0,35,0,34,34,34,34,34,35,34,35,34,35,34,36,0,0,36,0,37,36,36,37,36,37,37,36,36,36,36,36,36,36,36,37,36,37,36,36,37,37,36,36,36,37,37,37,37,36,36,36,37,36,37,36,37,36,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],"f":[0,0,0,0,0,0,0,0,0,0,0,0,[[1,2],3],[[1,4],5],[1,5],[1],[[1,6],5],[[1,3],7],[[]],[[]],[[1,3]],[[1,3]],[[1,3]],[[1,3,[8,[3]]],5],[[1,3,[8,[3]]],5],[[1,2],3],0,0,[[1,2]],0,[[1,3,9],10],[[]],[[1,2],[[11,[3]]]],[[1,6],[[11,[5]]]],[[]],[[1,3]],[[1,[12,[3]]],[[12,[3]]]],[[],1],0,0,[[1,3]],[[1,13,[15,[14]]],7],[[1,3,16]],[[1,3,16]],[[1,3,16,16]],[[1,3,2]],[[1,2]],0,[[],17],[[],17],[[],18],0,[[1,6],3],[[1,6]],[[1,19],12],0,0,0,[[20,13,13]],[[]],[[]],[[21,[22,[5,5]]],[[12,[[12,[5]]]]]],0,[20,6],[[13,13],6],[20],[[20,9],10],[21,20],[[]],[[]],[20,[[11,[6]]]],[[20,[12,[5]]],6],[[],6],[[],17],[[],17],[[],18],0,0,0,0,0,0,0,[[]],[[]],[2,23],[2,[[8,[3]]]],[2,2],[[]],[[2,2],24],[[],24],[[2,9],10],[[]],[[2,25]],[[]],[[]],[[],17],[[],17],[[],18],0,[[7,1],21],[[7,1]],[[]],[[]],[[],7],[[]],[[]],[[[26,[5]],3],7],[[1,3]],0,[[7,1],[[12,[3]]]],0,[[7,1],12],[[7,1]],[[7,1]],[[7,1]],[[7,1]],[[],17],[[],17],[[],18],0,0,[[]],[[]],[[21,9],10],0,[[]],[[]],[[],6],[[],17],[[],17],[[],18],0,0,0,0,[[]],[[]],[4,4],[[]],[[4,4],24],[[],24],[[4,9],10],[[4,9],10],[[]],[[4,25]],[[]],[[]],[[],6],[[],17],[[],17],[[],18],0,0,0,0,[[13,[15,[14]]],7],[[13,1],7],[13],[[[11,[6]]],[[15,[14]]]],[6,6],0,0,0,[[]],[[]],0,[[]],[[]],[6,27],[[27,13,1],7],[[27,6],6],[[],17],[[],17],[[],18],0,0,0,0,0,0,[[]],[[]],[[]],[[]],0,[28,28],[[]],[[28,28],29],0,[[28,28],24],[[],24],0,[[28,9],10],[[]],[[]],[[28,25]],[[]],[[]],0,0,0,0,0,[[28,13],[[17,[[30,[28]],[31,[28]]]]]],[[[32,[28]],1,[26,[5]]],[[12,[3]]]],[[13,1],7],[[33,13,1],7],[[[32,[28]],1,[26,[5]]],3],[[28,28],[[11,[29]]]],[[]],[[],17],[[],17],[[],17],[[],17],[[],18],[[],18],0,0,0,0,0,0,0,0,[[]],[[]],[[]],[[]],[34,34],[[]],[[34,34],29],0,0,0,[[34,34],24],[[],24],0,0,[[34,9],10],[[]],[[]],[[34,25]],[[]],[[]],0,0,0,[[34,13],[[17,[[30,[34]],[31,[34]]]]]],[[[32,[34]],[8,[3]],1],[[12,[3]]]],[[35,13,1],7],[[[32,[34]],[8,[3]],1],3],[[34,34],[[11,[29]]]],0,0,[[]],[[],17],[[],17],[[],17],[[],17],[[],18],[[],18],0,0,0,0,0,0,0,0,[[]],[[]],[[]],[[]],[6,24],[36,36],[[]],[[36,36],29],[[36,36],24],[[],24],0,0,[[36,9],10],0,[[]],[[]],[[36,25]],[[]],[[]],[[[12,[7]],24],37],0,0,0,[[36,13],[[17,[[30,[36]],[31,[36]]]]]],[[37,[32,[36]],1],[[12,[3]]]],[[37,6,1],7],[[37,[32,[36]],1],3],[[36,36],[[11,[29]]]],[[]],[[],17],[[],17],[[],17],[[],17],[[],18],[[],18],0,0,0,0,0,[[[12,[6]]]],[13,6],[13,6],0,0,0,0,0,0,0,[13,24],[13],[13],[13,6],[[13,13,13,[8,[13]]],6],[13,[[11,[[12,[5]]]]]],[13,6]],"c":[],"p":[[3,"Arena"],[4,"Expr"],[15,"usize"],[4,"Var"],[15,"i32"],[3,"String"],[3,"Formula"],[15,"slice"],[3,"Formatter"],[6,"Result"],[4,"Option"],[3,"Vec"],[15,"str"],[8,"FormulaParser"],[3,"Box"],[8,"FnMut"],[4,"Result"],[3,"TypeId"],[8,"Fn"],[3,"Clauses"],[3,"FormulaRef"],[3,"HashMap"],[15,"u64"],[15,"bool"],[8,"Hasher"],[3,"HashSet"],[3,"IoFormulaParser"],[4,"Rule"],[4,"Ordering"],[3,"Pairs"],[3,"Error"],[3,"Pair"],[3,"ModelFormulaParser"],[4,"Rule"],[3,"SatFormulaParser"],[4,"Rule"],[3,"SatInlineFormulaParser"],[8,"FormulaParsee"]]}\
}');
if (typeof window !== 'undefined' && window.initSearch) {window.initSearch(searchIndex)};
if (typeof exports !== 'undefined') {exports.searchIndex = searchIndex};
