//! Exhaustiveness and redundancy checking for pattern matching.
//!
//! Implements Maranget's algorithm from "Warnings for Pattern Matching" (JFP 2007).
//!
//! Key concepts:
//! - A pattern matrix represents all remaining cases to match
//! - Exhaustiveness: check if any case could be missed
//! - Redundancy: check if any arm can never match
//!
//! The algorithm works by:
//! 1. Building a simplified pattern matrix from match arms
//! 2. Recursively specializing/defaulting the matrix
//! 3. Tracking which constructors are covered

use crate::adt::AdtRegistry;
use crate::infer::ty::Ty;
use std::collections::HashSet;
use strata_ast::span::Span;

/// Maximum size of pattern matrix (rows × columns) to prevent DoS
const MAX_PATTERN_MATRIX_SIZE: usize = 10_000;

/// Maximum recursion depth for exhaustiveness checking
const MAX_EXHAUSTIVENESS_DEPTH: usize = 100;

/// Errors that can occur during exhaustiveness checking
#[derive(Debug, Clone)]
pub enum ExhaustivenessError {
    /// Match is not exhaustive - returns a witness (example of uncovered case)
    NonExhaustive { witness: Witness, span: Span },
    /// Pattern matrix too large (DoS protection)
    MatrixTooLarge { size: usize, span: Span },
    /// Recursion depth exceeded (DoS protection)
    DepthExceeded { span: Span },
}

/// A witness is an example of an uncovered pattern
#[derive(Debug, Clone)]
pub struct Witness {
    /// The pattern that isn't covered
    pub patterns: Vec<WitnessPat>,
}

impl Witness {
    pub fn wildcard() -> Self {
        Witness {
            patterns: vec![WitnessPat::Wildcard],
        }
    }

    pub fn single(pat: WitnessPat) -> Self {
        Witness {
            patterns: vec![pat],
        }
    }

    pub fn from_patterns(patterns: Vec<WitnessPat>) -> Self {
        Witness { patterns }
    }
}

impl std::fmt::Display for Witness {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.patterns.is_empty() {
            write!(f, "_")
        } else if self.patterns.len() == 1 {
            write!(f, "{}", self.patterns[0])
        } else {
            write!(f, "(")?;
            for (i, pat) in self.patterns.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}", pat)?;
            }
            write!(f, ")")
        }
    }
}

/// A pattern in a witness
#[derive(Debug, Clone)]
pub enum WitnessPat {
    /// Wildcard pattern
    Wildcard,
    /// Constructor pattern (enum variant or struct)
    Constructor { name: String, args: Vec<WitnessPat> },
    /// Literal pattern
    Literal(String),
}

impl std::fmt::Display for WitnessPat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WitnessPat::Wildcard => write!(f, "_"),
            WitnessPat::Constructor { name, args } => {
                write!(f, "{}", name)?;
                if !args.is_empty() {
                    write!(f, "(")?;
                    for (i, arg) in args.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{}", arg)?;
                    }
                    write!(f, ")")?;
                }
                Ok(())
            }
            WitnessPat::Literal(s) => write!(f, "{}", s),
        }
    }
}

/// Simplified pattern for exhaustiveness checking.
/// This is a normalized representation that's easier to work with than AST patterns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SimplifiedPat {
    /// Wildcard pattern (matches anything)
    Wildcard,
    /// Constructor pattern (enum variant, struct, or tuple)
    Constructor {
        /// Fully qualified constructor name (e.g., "Option::Some", "Point", "Tuple2")
        name: String,
        /// Constructor arguments (sub-patterns)
        args: Vec<SimplifiedPat>,
    },
    /// Literal pattern (Int, Bool, String)
    Literal(LiteralPat),
}

/// Literal patterns
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LiteralPat {
    Int(i64),
    Bool(bool),
    String(String),
}

impl std::fmt::Display for LiteralPat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LiteralPat::Int(n) => write!(f, "{}", n),
            LiteralPat::Bool(b) => write!(f, "{}", b),
            LiteralPat::String(s) => write!(f, "\"{}\"", s),
        }
    }
}

/// A row in the pattern matrix
#[derive(Debug, Clone)]
pub struct PatternRow {
    /// Patterns in this row (one per column)
    pub patterns: Vec<SimplifiedPat>,
    /// Index of the original match arm (for redundancy reporting)
    pub arm_index: usize,
}

impl PatternRow {
    pub fn new(patterns: Vec<SimplifiedPat>, arm_index: usize) -> Self {
        PatternRow {
            patterns,
            arm_index,
        }
    }

    /// Get the first pattern in the row
    pub fn first(&self) -> Option<&SimplifiedPat> {
        self.patterns.first()
    }

    /// Get all patterns after the first
    pub fn rest(&self) -> &[SimplifiedPat] {
        if self.patterns.is_empty() {
            &[]
        } else {
            &self.patterns[1..]
        }
    }
}

/// A pattern matrix for exhaustiveness checking
#[derive(Debug, Clone)]
pub struct PatternMatrix {
    /// Rows of patterns
    pub rows: Vec<PatternRow>,
    /// Types of each column
    pub column_types: Vec<Ty>,
}

impl PatternMatrix {
    pub fn new(column_types: Vec<Ty>) -> Self {
        PatternMatrix {
            rows: vec![],
            column_types,
        }
    }

    pub fn with_rows(rows: Vec<PatternRow>, column_types: Vec<Ty>) -> Self {
        PatternMatrix { rows, column_types }
    }

    /// Add a row to the matrix
    pub fn add_row(&mut self, row: PatternRow) {
        self.rows.push(row);
    }

    /// Number of rows
    pub fn num_rows(&self) -> usize {
        self.rows.len()
    }

    /// Number of columns
    pub fn num_columns(&self) -> usize {
        self.column_types.len()
    }

    /// Check if matrix is empty (no rows)
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Get the size (rows × columns) for DoS protection
    pub fn size(&self) -> usize {
        self.rows.len() * self.column_types.len().max(1)
    }

    /// Get the type of the first column
    pub fn first_column_type(&self) -> Option<&Ty> {
        self.column_types.first()
    }
}

/// Constructor information for exhaustiveness checking
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Constructor {
    /// Fully qualified name (e.g., "Option::Some", "true", "Point")
    pub name: String,
    /// Number of arguments
    pub arity: usize,
    /// Types of arguments (if known)
    pub arg_types: Vec<Ty>,
}

impl Constructor {
    pub fn new(name: impl Into<String>, arity: usize) -> Self {
        Constructor {
            name: name.into(),
            arity,
            arg_types: vec![],
        }
    }

    pub fn with_arg_types(name: impl Into<String>, arg_types: Vec<Ty>) -> Self {
        let arity = arg_types.len();
        Constructor {
            name: name.into(),
            arity,
            arg_types,
        }
    }
}

/// Context for exhaustiveness checking
pub struct ExhaustivenessChecker<'a> {
    /// ADT registry for looking up type information
    registry: &'a AdtRegistry,
    /// Current recursion depth
    depth: usize,
    /// Span for error reporting
    span: Span,
}

impl<'a> ExhaustivenessChecker<'a> {
    pub fn new(registry: &'a AdtRegistry, span: Span) -> Self {
        ExhaustivenessChecker {
            registry,
            depth: 0,
            span,
        }
    }

    /// Check if a pattern matrix is exhaustive.
    /// Returns None if exhaustive, Some(witness) if not.
    pub fn check_exhaustive(
        &mut self,
        matrix: &PatternMatrix,
    ) -> Result<Option<Witness>, ExhaustivenessError> {
        // DoS protection: check matrix size
        if matrix.size() > MAX_PATTERN_MATRIX_SIZE {
            return Err(ExhaustivenessError::MatrixTooLarge {
                size: matrix.size(),
                span: self.span,
            });
        }

        // DoS protection: check recursion depth
        if self.depth > MAX_EXHAUSTIVENESS_DEPTH {
            return Err(ExhaustivenessError::DepthExceeded { span: self.span });
        }

        self.depth += 1;
        let result = self.check_exhaustive_inner(matrix);
        self.depth -= 1;
        result
    }

    fn check_exhaustive_inner(
        &mut self,
        matrix: &PatternMatrix,
    ) -> Result<Option<Witness>, ExhaustivenessError> {
        // Base case 1: No columns - exhaustive iff there are rows
        if matrix.num_columns() == 0 {
            if matrix.is_empty() {
                // No rows, no columns - not exhaustive
                // Return empty witness (to be extended by callers)
                return Ok(Some(Witness::from_patterns(vec![])));
            } else {
                // Has rows, no columns - exhaustive
                return Ok(None);
            }
        }

        // Base case 2: Empty matrix with columns - not exhaustive
        if matrix.is_empty() {
            // Return a witness with wildcards for each column
            let witness = Witness::from_patterns(
                matrix
                    .column_types
                    .iter()
                    .map(|_| WitnessPat::Wildcard)
                    .collect(),
            );
            return Ok(Some(witness));
        }

        // Get constructors for the first column type
        let first_type = matrix.first_column_type().unwrap();
        let all_constructors = self.constructors_for_type(first_type);

        // Get constructors that appear in the first column
        let used_constructors = self.used_constructors(matrix);

        // Check if we have a complete signature (all constructors covered)
        let is_complete = self.is_complete_signature(&all_constructors, &used_constructors);

        if is_complete {
            // Complete signature: specialize on each constructor
            for ctor in &all_constructors {
                let specialized = self.specialize_matrix(matrix, ctor)?;
                if let Some(witness) = self.check_exhaustive(&specialized)? {
                    // Found a gap - reconstruct witness with this constructor
                    return Ok(Some(self.reconstruct_witness(ctor, witness)));
                }
            }
            // All constructors exhaustive
            Ok(None)
        } else {
            // Incomplete signature: check default matrix
            let default = self.default_matrix(matrix)?;
            if let Some(witness) = self.check_exhaustive(&default)? {
                // Found a gap - find a missing constructor
                let missing = self.find_missing_constructor(&all_constructors, &used_constructors);
                return Ok(Some(self.add_missing_constructor(missing, witness)));
            }
            Ok(None)
        }
    }

    /// Check which arms are redundant (unreachable).
    /// Returns indices of redundant arms.
    pub fn check_redundant(
        &mut self,
        matrix: &PatternMatrix,
    ) -> Result<Vec<usize>, ExhaustivenessError> {
        let mut redundant = vec![];

        // Check each row to see if it's useful against preceding rows
        for i in 0..matrix.rows.len() {
            // Build a matrix of all preceding rows
            let preceding: Vec<PatternRow> = matrix.rows[..i].to_vec();
            let preceding_matrix = PatternMatrix::with_rows(preceding, matrix.column_types.clone());

            // Get the current row as a single-row matrix
            let current_row = matrix.rows[i].clone();

            // Check if this row is useful (would match something not already covered)
            if !self.is_useful(&preceding_matrix, &current_row)? {
                redundant.push(matrix.rows[i].arm_index);
            }
        }

        Ok(redundant)
    }

    /// Check if a row is useful against a matrix (would match something new).
    fn is_useful(
        &mut self,
        matrix: &PatternMatrix,
        row: &PatternRow,
    ) -> Result<bool, ExhaustivenessError> {
        // DoS protection
        if matrix.size() > MAX_PATTERN_MATRIX_SIZE {
            return Err(ExhaustivenessError::MatrixTooLarge {
                size: matrix.size(),
                span: self.span,
            });
        }

        if self.depth > MAX_EXHAUSTIVENESS_DEPTH {
            return Err(ExhaustivenessError::DepthExceeded { span: self.span });
        }

        self.depth += 1;
        let result = self.is_useful_inner(matrix, row);
        self.depth -= 1;
        result
    }

    fn is_useful_inner(
        &mut self,
        matrix: &PatternMatrix,
        row: &PatternRow,
    ) -> Result<bool, ExhaustivenessError> {
        // Base case: No columns - useful iff matrix is empty
        if matrix.num_columns() == 0 || row.patterns.is_empty() {
            return Ok(matrix.is_empty());
        }

        let first_pat = row.first().unwrap();
        let first_type = matrix.first_column_type().unwrap();

        match first_pat {
            SimplifiedPat::Wildcard => {
                // Wildcard: need to check if useful for any constructor
                let all_constructors = self.constructors_for_type(first_type);
                let used_constructors = self.used_constructors(matrix);

                if self.is_complete_signature(&all_constructors, &used_constructors) {
                    // Complete signature: check each constructor
                    for ctor in &all_constructors {
                        let specialized_matrix = self.specialize_matrix(matrix, ctor)?;
                        let specialized_row = self.specialize_row(row, ctor);
                        if self.is_useful(&specialized_matrix, &specialized_row)? {
                            return Ok(true);
                        }
                    }
                    Ok(false)
                } else {
                    // Incomplete signature: check default matrix
                    let default = self.default_matrix(matrix)?;
                    let default_row = self.default_row(row);
                    self.is_useful(&default, &default_row)
                }
            }

            SimplifiedPat::Constructor { name, args } => {
                // Constructor: specialize matrix and row on this constructor
                // Look up the constructor from the type to get proper arg_types
                let ctor = self
                    .lookup_constructor(first_type, name, args.len())
                    .unwrap_or_else(|| Constructor::new(name.clone(), args.len()));
                let specialized_matrix = self.specialize_matrix(matrix, &ctor)?;
                let specialized_row = self.specialize_row(row, &ctor);
                self.is_useful(&specialized_matrix, &specialized_row)
            }

            SimplifiedPat::Literal(lit) => {
                // Literal: treat as a nullary constructor
                let ctor = Constructor::new(format!("{}", lit), 0);
                let specialized_matrix = self.specialize_matrix(matrix, &ctor)?;
                let specialized_row = self.specialize_row(row, &ctor);
                self.is_useful(&specialized_matrix, &specialized_row)
            }
        }
    }

    /// Get all constructors for a type
    fn constructors_for_type(&self, ty: &Ty) -> Vec<Constructor> {
        match ty {
            Ty::Const(c) => {
                use crate::infer::ty::TyConst;
                match c {
                    TyConst::Bool => {
                        vec![Constructor::new("true", 0), Constructor::new("false", 0)]
                    }
                    // Int, Float, String have infinite constructors
                    TyConst::Int | TyConst::Float | TyConst::String | TyConst::Unit => vec![],
                }
            }

            Ty::Adt { name, args } => {
                // Look up in registry
                if let Some(adt) = self.registry.get(name) {
                    if adt.is_enum() {
                        // Enum: return all variants
                        adt.variants()
                            .map(|variants| {
                                variants
                                    .iter()
                                    .map(|v| {
                                        let full_name = format!("{}::{}", name, v.name);
                                        let arg_types = match &v.fields {
                                            crate::adt::VariantFields::Unit => vec![],
                                            crate::adt::VariantFields::Tuple(tys) => {
                                                // Substitute type parameters
                                                tys.iter()
                                                    .map(|t| self.substitute_type_args(t, args))
                                                    .collect()
                                            }
                                        };
                                        Constructor::with_arg_types(full_name, arg_types)
                                    })
                                    .collect()
                            })
                            .unwrap_or_default()
                    } else {
                        // Struct: single constructor (the struct itself)
                        let arg_types = adt
                            .fields()
                            .map(|fields| {
                                fields
                                    .iter()
                                    .map(|f| self.substitute_type_args(&f.ty, args))
                                    .collect()
                            })
                            .unwrap_or_default();
                        vec![Constructor::with_arg_types(name.clone(), arg_types)]
                    }
                } else {
                    // Unknown ADT - treat as having infinite constructors
                    vec![]
                }
            }

            Ty::Tuple(tys) => {
                // Tuple: single constructor
                let name = format!("Tuple{}", tys.len());
                vec![Constructor::with_arg_types(name, tys.clone())]
            }

            // Type variables, Never, Arrow, List - no known constructors
            _ => vec![],
        }
    }

    /// Look up a constructor by name from a type's constructors.
    /// Returns the constructor with proper arg_types populated.
    fn lookup_constructor(&self, ty: &Ty, name: &str, arity: usize) -> Option<Constructor> {
        let all_ctors = self.constructors_for_type(ty);
        all_ctors
            .into_iter()
            .find(|c| c.name == name && c.arity == arity)
    }

    /// Substitute type arguments in a type
    #[allow(clippy::only_used_in_recursion)]
    fn substitute_type_args(&self, ty: &Ty, args: &[Ty]) -> Ty {
        match ty {
            Ty::Var(v) => {
                // Type variable: substitute if in range
                let idx = v.0 as usize;
                if idx < args.len() {
                    args[idx].clone()
                } else {
                    ty.clone()
                }
            }
            Ty::Arrow(params, ret, eff) => Ty::arrow_eff(
                params
                    .iter()
                    .map(|t| self.substitute_type_args(t, args))
                    .collect(),
                self.substitute_type_args(ret, args),
                *eff,
            ),
            Ty::Tuple(tys) => Ty::Tuple(
                tys.iter()
                    .map(|t| self.substitute_type_args(t, args))
                    .collect(),
            ),
            Ty::List(t) => Ty::List(Box::new(self.substitute_type_args(t, args))),
            Ty::Adt {
                name,
                args: inner_args,
            } => Ty::Adt {
                name: name.clone(),
                args: inner_args
                    .iter()
                    .map(|t| self.substitute_type_args(t, args))
                    .collect(),
            },
            _ => ty.clone(),
        }
    }

    /// Get constructors used in the first column of the matrix
    fn used_constructors(&self, matrix: &PatternMatrix) -> HashSet<String> {
        let mut used = HashSet::new();
        for row in &matrix.rows {
            if let Some(pat) = row.first() {
                match pat {
                    SimplifiedPat::Constructor { name, .. } => {
                        used.insert(name.clone());
                    }
                    SimplifiedPat::Literal(lit) => {
                        used.insert(format!("{}", lit));
                    }
                    SimplifiedPat::Wildcard => {}
                }
            }
        }
        used
    }

    /// Check if the used constructors form a complete signature
    fn is_complete_signature(&self, all: &[Constructor], used: &HashSet<String>) -> bool {
        // If all constructors is empty, signature is incomplete (infinite type)
        if all.is_empty() {
            return false;
        }
        // Complete if all constructors are used
        all.iter().all(|c| used.contains(&c.name))
    }

    /// Find a constructor not in the used set
    fn find_missing_constructor(
        &self,
        all: &[Constructor],
        used: &HashSet<String>,
    ) -> Option<Constructor> {
        all.iter().find(|c| !used.contains(&c.name)).cloned()
    }

    /// Specialize a matrix on a constructor.
    /// Keeps only rows where the first pattern matches the constructor,
    /// and expands the constructor's arguments.
    fn specialize_matrix(
        &self,
        matrix: &PatternMatrix,
        ctor: &Constructor,
    ) -> Result<PatternMatrix, ExhaustivenessError> {
        let mut new_rows = vec![];

        for row in &matrix.rows {
            if let Some(specialized) = self.specialize_row_on_ctor(row, ctor) {
                new_rows.push(specialized);
            }
        }

        // New column types: constructor's arg types + remaining original types
        let mut new_column_types = ctor.arg_types.clone();
        if matrix.column_types.len() > 1 {
            new_column_types.extend(matrix.column_types[1..].to_vec());
        }

        Ok(PatternMatrix::with_rows(new_rows, new_column_types))
    }

    /// Specialize a single row on a constructor
    fn specialize_row_on_ctor(&self, row: &PatternRow, ctor: &Constructor) -> Option<PatternRow> {
        let first = row.first()?;

        match first {
            SimplifiedPat::Wildcard => {
                // Wildcard matches any constructor: expand with wildcards
                let mut new_patterns: Vec<SimplifiedPat> =
                    (0..ctor.arity).map(|_| SimplifiedPat::Wildcard).collect();
                new_patterns.extend(row.rest().to_vec());
                Some(PatternRow::new(new_patterns, row.arm_index))
            }

            SimplifiedPat::Constructor { name, args } if name == &ctor.name => {
                // Matching constructor: expand args
                let mut new_patterns = args.clone();
                new_patterns.extend(row.rest().to_vec());
                Some(PatternRow::new(new_patterns, row.arm_index))
            }

            SimplifiedPat::Literal(lit) if format!("{}", lit) == ctor.name => {
                // Matching literal: no args to expand
                let new_patterns = row.rest().to_vec();
                Some(PatternRow::new(new_patterns, row.arm_index))
            }

            _ => {
                // Non-matching constructor
                None
            }
        }
    }

    /// Specialize a row for usefulness checking
    fn specialize_row(&self, row: &PatternRow, ctor: &Constructor) -> PatternRow {
        let first = row.first();

        match first {
            Some(SimplifiedPat::Wildcard) => {
                // Expand wildcard with wildcards for constructor args
                let mut new_patterns: Vec<SimplifiedPat> =
                    (0..ctor.arity).map(|_| SimplifiedPat::Wildcard).collect();
                new_patterns.extend(row.rest().to_vec());
                PatternRow::new(new_patterns, row.arm_index)
            }

            Some(SimplifiedPat::Constructor { args, .. }) => {
                // Use the constructor's args
                let mut new_patterns = args.clone();
                new_patterns.extend(row.rest().to_vec());
                PatternRow::new(new_patterns, row.arm_index)
            }

            Some(SimplifiedPat::Literal(_)) => {
                // Literal has no args
                PatternRow::new(row.rest().to_vec(), row.arm_index)
            }

            None => PatternRow::new(vec![], row.arm_index),
        }
    }

    /// Build the default matrix (rows that start with wildcard).
    /// Removes the first column.
    fn default_matrix(&self, matrix: &PatternMatrix) -> Result<PatternMatrix, ExhaustivenessError> {
        let mut new_rows = vec![];

        for row in &matrix.rows {
            if let Some(first) = row.first() {
                if matches!(first, SimplifiedPat::Wildcard) {
                    new_rows.push(PatternRow::new(row.rest().to_vec(), row.arm_index));
                }
            }
        }

        // Remove first column type
        let new_column_types = if matrix.column_types.len() > 1 {
            matrix.column_types[1..].to_vec()
        } else {
            vec![]
        };

        Ok(PatternMatrix::with_rows(new_rows, new_column_types))
    }

    /// Build default row for usefulness checking
    fn default_row(&self, row: &PatternRow) -> PatternRow {
        PatternRow::new(row.rest().to_vec(), row.arm_index)
    }

    /// Reconstruct a witness by prepending a constructor
    fn reconstruct_witness(&self, ctor: &Constructor, inner: Witness) -> Witness {
        let (ctor_args, rest): (Vec<_>, Vec<_>) = inner
            .patterns
            .into_iter()
            .enumerate()
            .partition(|(i, _)| *i < ctor.arity);

        let args: Vec<WitnessPat> = ctor_args.into_iter().map(|(_, p)| p).collect();
        let rest: Vec<WitnessPat> = rest.into_iter().map(|(_, p)| p).collect();

        let ctor_pat = WitnessPat::Constructor {
            name: ctor.name.clone(),
            args,
        };

        let mut patterns = vec![ctor_pat];
        patterns.extend(rest);

        Witness::from_patterns(patterns)
    }

    /// Add a missing constructor to a witness
    fn add_missing_constructor(&self, missing: Option<Constructor>, inner: Witness) -> Witness {
        match missing {
            Some(ctor) => {
                let ctor_pat = WitnessPat::Constructor {
                    name: ctor.name.clone(),
                    args: (0..ctor.arity).map(|_| WitnessPat::Wildcard).collect(),
                };

                let mut patterns = vec![ctor_pat];
                patterns.extend(inner.patterns);
                Witness::from_patterns(patterns)
            }
            None => {
                // No known missing constructor - use wildcard
                let mut patterns = vec![WitnessPat::Wildcard];
                patterns.extend(inner.patterns);
                Witness::from_patterns(patterns)
            }
        }
    }
}

/// Convert an AST pattern to a SimplifiedPat for exhaustiveness checking.
/// This requires the resolved scrutinee type to properly handle variant patterns.
/// Note: registry is passed through for potential future use in complex pattern simplification.
#[allow(clippy::only_used_in_recursion)]
pub fn simplify_pattern(pat: &strata_ast::ast::Pat, registry: &AdtRegistry) -> SimplifiedPat {
    use strata_ast::ast::Pat;

    match pat {
        Pat::Wildcard(_) => SimplifiedPat::Wildcard,

        Pat::Ident(_) => {
            // Variable bindings act as wildcards for exhaustiveness
            SimplifiedPat::Wildcard
        }

        Pat::Literal(lit, _) => {
            use strata_ast::ast::Lit;
            let lit_pat = match lit {
                Lit::Int(n) => LiteralPat::Int(*n),
                Lit::Bool(b) => LiteralPat::Bool(*b),
                Lit::Str(s) => LiteralPat::String(s.clone()),
                Lit::Float(_) => {
                    // Floats are tricky for pattern matching - treat as wildcard
                    return SimplifiedPat::Wildcard;
                }
                Lit::Nil => {
                    // Nil matches Unit - treat as a unit constructor
                    return SimplifiedPat::Constructor {
                        name: "()".to_string(),
                        args: vec![],
                    };
                }
            };
            SimplifiedPat::Literal(lit_pat)
        }

        Pat::Tuple(pats, _) => {
            let args: Vec<SimplifiedPat> =
                pats.iter().map(|p| simplify_pattern(p, registry)).collect();
            let name = format!("Tuple{}", args.len());
            SimplifiedPat::Constructor { name, args }
        }

        Pat::Variant { path, fields, .. } => {
            // Build fully qualified name
            let name = path
                .segments
                .iter()
                .map(|s| s.text.as_str())
                .collect::<Vec<_>>()
                .join("::");

            let args: Vec<SimplifiedPat> = fields
                .iter()
                .map(|p| simplify_pattern(p, registry))
                .collect();

            SimplifiedPat::Constructor { name, args }
        }

        Pat::Struct { path, fields, .. } => {
            // Build fully qualified name
            let name = path
                .segments
                .iter()
                .map(|s| s.text.as_str())
                .collect::<Vec<_>>()
                .join("::");

            // For struct patterns, we need to order fields consistently
            // For simplicity, convert field patterns in order
            let args: Vec<SimplifiedPat> = fields
                .iter()
                .map(|f| simplify_pattern(&f.pat, registry))
                .collect();

            SimplifiedPat::Constructor { name, args }
        }
    }
}

/// Build a PatternMatrix from match arms and the scrutinee type.
pub fn build_pattern_matrix(
    arms: &[strata_ast::ast::MatchArm],
    scrutinee_ty: &Ty,
    registry: &AdtRegistry,
) -> PatternMatrix {
    let column_types = vec![scrutinee_ty.clone()];
    let mut matrix = PatternMatrix::new(column_types);

    for (i, arm) in arms.iter().enumerate() {
        let pat = simplify_pattern(&arm.pat, registry);
        matrix.add_row(PatternRow::new(vec![pat], i));
    }

    matrix
}

/// Check exhaustiveness and redundancy for a match expression.
/// Returns (non_exhaustive_witness, redundant_arm_indices).
pub fn check_match(
    arms: &[strata_ast::ast::MatchArm],
    scrutinee_ty: &Ty,
    registry: &AdtRegistry,
    span: Span,
) -> Result<(Option<Witness>, Vec<usize>), ExhaustivenessError> {
    let matrix = build_pattern_matrix(arms, scrutinee_ty, registry);
    let mut checker = ExhaustivenessChecker::new(registry, span);

    let witness = checker.check_exhaustive(&matrix)?;
    let redundant = checker.check_redundant(&matrix)?;

    Ok((witness, redundant))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_registry() -> AdtRegistry {
        AdtRegistry::new()
    }

    fn span() -> Span {
        Span { start: 0, end: 0 }
    }

    #[test]
    fn test_empty_matrix_not_exhaustive() {
        let registry = empty_registry();
        let mut checker = ExhaustivenessChecker::new(&registry, span());

        let matrix = PatternMatrix::new(vec![Ty::int()]);
        let result = checker.check_exhaustive(&matrix).unwrap();

        assert!(result.is_some());
    }

    #[test]
    fn test_wildcard_exhaustive() {
        let registry = empty_registry();
        let mut checker = ExhaustivenessChecker::new(&registry, span());

        let mut matrix = PatternMatrix::new(vec![Ty::int()]);
        matrix.add_row(PatternRow::new(vec![SimplifiedPat::Wildcard], 0));

        let result = checker.check_exhaustive(&matrix).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_bool_exhaustive() {
        let registry = empty_registry();
        let mut checker = ExhaustivenessChecker::new(&registry, span());

        let mut matrix = PatternMatrix::new(vec![Ty::bool_()]);
        matrix.add_row(PatternRow::new(
            vec![SimplifiedPat::Literal(LiteralPat::Bool(true))],
            0,
        ));
        matrix.add_row(PatternRow::new(
            vec![SimplifiedPat::Literal(LiteralPat::Bool(false))],
            1,
        ));

        let result = checker.check_exhaustive(&matrix).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_bool_not_exhaustive() {
        let registry = empty_registry();
        let mut checker = ExhaustivenessChecker::new(&registry, span());

        let mut matrix = PatternMatrix::new(vec![Ty::bool_()]);
        matrix.add_row(PatternRow::new(
            vec![SimplifiedPat::Literal(LiteralPat::Bool(true))],
            0,
        ));

        let result = checker.check_exhaustive(&matrix).unwrap();
        assert!(result.is_some());
        let witness = result.unwrap();
        assert_eq!(format!("{}", witness), "false");
    }

    #[test]
    fn test_redundant_after_wildcard() {
        let registry = empty_registry();
        let mut checker = ExhaustivenessChecker::new(&registry, span());

        let mut matrix = PatternMatrix::new(vec![Ty::int()]);
        matrix.add_row(PatternRow::new(vec![SimplifiedPat::Wildcard], 0));
        matrix.add_row(PatternRow::new(
            vec![SimplifiedPat::Literal(LiteralPat::Int(42))],
            1,
        ));

        let redundant = checker.check_redundant(&matrix).unwrap();
        assert_eq!(redundant, vec![1]);
    }

    #[test]
    fn test_no_redundant_patterns() {
        let registry = empty_registry();
        let mut checker = ExhaustivenessChecker::new(&registry, span());

        let mut matrix = PatternMatrix::new(vec![Ty::bool_()]);
        matrix.add_row(PatternRow::new(
            vec![SimplifiedPat::Literal(LiteralPat::Bool(true))],
            0,
        ));
        matrix.add_row(PatternRow::new(
            vec![SimplifiedPat::Literal(LiteralPat::Bool(false))],
            1,
        ));

        let redundant = checker.check_redundant(&matrix).unwrap();
        assert!(redundant.is_empty());
    }

    #[test]
    fn test_witness_display() {
        let witness = Witness::from_patterns(vec![
            WitnessPat::Constructor {
                name: "Option::None".to_string(),
                args: vec![],
            },
            WitnessPat::Wildcard,
        ]);
        assert_eq!(format!("{}", witness), "(Option::None, _)");

        let witness = Witness::single(WitnessPat::Constructor {
            name: "Some".to_string(),
            args: vec![WitnessPat::Wildcard],
        });
        assert_eq!(format!("{}", witness), "Some(_)");
    }
}
