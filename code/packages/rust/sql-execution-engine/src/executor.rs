//! Executor — the relational pipeline for a SELECT statement.
//!
//! # Execution Order
//!
//! SQL has a logical evaluation order that differs from the syntactic order:
//!
//! ```text
//! 1. FROM        — scan source table
//! 2. JOIN        — combine tables
//! 3. WHERE       — filter rows
//! 4. GROUP BY    — group rows for aggregation
//! 5. HAVING      — filter groups
//! 6. SELECT      — project columns
//! 7. DISTINCT    — deduplicate
//! 8. ORDER BY    — sort
//! 9. LIMIT       — paginate
//! ```

use std::collections::HashMap;

use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};

use crate::aggregate::compute_aggregates;
use crate::data_source::{DataSource, SqlPrimitive, SqlValue};
use crate::errors::ExecutionError;
use crate::expression::{eval_expr, node_text};
use crate::join::perform_join;
use crate::result::QueryResult;

/// Execute a `select_stmt` AST node against a data source.
pub fn execute_select(
    stmt: &GrammarASTNode,
    source: &dyn DataSource,
) -> Result<QueryResult, ExecutionError> {
    // --- Phase 1: FROM ---
    let table_ref = find_child(stmt, "table_ref");
    let (table_name, table_alias) = extract_table_ref(table_ref);
    let raw_rows = source.scan(&table_name)?;
    let effective_alias = if table_alias.is_empty() { table_name.clone() } else { table_alias.clone() };
    let mut rows = qualify_rows(raw_rows, &effective_alias);

    // --- Phase 2: JOIN ---
    let join_clauses: Vec<&GrammarASTNode> = find_children(stmt, "join_clause");
    for jc in join_clauses {
        rows = process_join(rows, jc, source)?;
    }

    // --- Phase 3: WHERE ---
    if let Some(where_clause) = find_child(stmt, "where_clause") {
        rows = apply_where(rows, where_clause)?;
    }

    // --- Phase 4 & 5: GROUP BY / HAVING / Aggregates ---
    let group_clause = find_child(stmt, "group_clause");
    let having_clause = find_child(stmt, "having_clause");
    let select_list = find_child(stmt, "select_list");

    let mut all_agg_specs: Vec<(String, String)> = Vec::new();
    if let Some(sl) = select_list {
        collect_agg_specs_node(sl, &mut all_agg_specs);
    }
    if let Some(hc) = having_clause {
        collect_agg_specs_node(hc, &mut all_agg_specs);
    }
    all_agg_specs.dedup();

    if group_clause.is_some() || !all_agg_specs.is_empty() {
        rows = apply_group_by_and_aggregate(rows, group_clause, having_clause, &all_agg_specs)?;
    }

    // --- Phase 6: ORDER BY (before projection so non-selected cols accessible) ---
    if let Some(order_clause) = find_child(stmt, "order_clause") {
        rows = apply_order_by(rows, order_clause)?;
    }

    // --- Phase 7: SELECT projection ---
    let has_distinct = has_distinct_qualifier(stmt);
    let (columns, mut rows) = apply_select(select_list, rows, &table_name, &effective_alias, source)?;

    // --- Phase 8: DISTINCT ---
    if has_distinct {
        rows = apply_distinct(rows);
    }

    // --- Phase 9: LIMIT ---
    if let Some(limit_clause) = find_child(stmt, "limit_clause") {
        rows = apply_limit(rows, limit_clause);
    }

    Ok(QueryResult { columns, rows })
}

// ---------------------------------------------------------------------------
// Phase 1: FROM
// ---------------------------------------------------------------------------

fn extract_table_ref(table_ref: Option<&GrammarASTNode>) -> (String, String) {
    let Some(tr) = table_ref else { return (String::new(), String::new()); };

    let table_name = if let Some(tn_node) = find_child(tr, "table_name") {
        first_token_value(tn_node)
    } else {
        first_token_value(tr)
    };

    let mut alias = String::new();
    let children = &tr.children;
    for (i, child) in children.iter().enumerate() {
        if keyword_value_of(child) == "AS" {
            if let Some(next) = children.get(i + 1) {
                alias = token_value_of(next);
            }
        }
    }

    (table_name, alias)
}

fn qualify_rows(rows: Vec<HashMap<String, SqlValue>>, alias: &str) -> Vec<HashMap<String, SqlValue>> {
    rows.into_iter().map(|row| {
        let mut qrow = row.clone();
        for (key, val) in &row {
            if !key.contains('.') {
                qrow.insert(format!("{alias}.{key}"), val.clone());
            }
        }
        qrow
    }).collect()
}

// ---------------------------------------------------------------------------
// Phase 2: JOIN
// ---------------------------------------------------------------------------

fn process_join(
    left_rows: Vec<HashMap<String, SqlValue>>,
    join_clause: &GrammarASTNode,
    source: &dyn DataSource,
) -> Result<Vec<HashMap<String, SqlValue>>, ExecutionError> {
    let join_type_node = find_child(join_clause, "join_type");
    let join_type = extract_join_type(join_type_node);

    let table_ref = find_child(join_clause, "table_ref");
    let (right_table, right_alias) = extract_table_ref(table_ref);
    let effective_alias = if right_alias.is_empty() { right_table.clone() } else { right_alias.clone() };

    let right_raw = source.scan(&right_table)?;
    let right_rows = qualify_rows(right_raw, &effective_alias);

    let on_condition = find_on_condition(join_clause);

    perform_join(&left_rows, &right_rows, &effective_alias, &join_type, on_condition)
}

fn extract_join_type(node: Option<&GrammarASTNode>) -> String {
    let Some(n) = node else { return "INNER".to_string(); };
    let keywords: Vec<String> = n.children.iter()
        .filter_map(|c| {
            if let ASTNodeOrToken::Token(tok) = c { Some(tok.value.to_uppercase()) }
            else { None }
        })
        .collect();
    if keywords.is_empty() { "INNER".to_string() } else { keywords.join(" ") }
}

fn find_on_condition(join_clause: &GrammarASTNode) -> Option<&ASTNodeOrToken> {
    let children = &join_clause.children;
    for (i, child) in children.iter().enumerate() {
        if keyword_value_of(child) == "ON" {
            return children.get(i + 1);
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Phase 3: WHERE
// ---------------------------------------------------------------------------

fn apply_where(
    rows: Vec<HashMap<String, SqlValue>>,
    where_clause: &GrammarASTNode,
) -> Result<Vec<HashMap<String, SqlValue>>, ExecutionError> {
    let expr = find_expr_in_clause(where_clause);
    let Some(expr) = expr else { return Ok(rows); };
    rows.into_iter()
        .filter_map(|row| {
            match eval_expr(expr, &row) {
                Ok(val) => {
                    if is_truthy(&val) { Some(Ok(row)) } else { None }
                }
                Err(e) => Some(Err(e)),
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Phase 4 & 5: GROUP BY / HAVING
// ---------------------------------------------------------------------------

fn apply_group_by_and_aggregate(
    rows: Vec<HashMap<String, SqlValue>>,
    group_clause: Option<&GrammarASTNode>,
    having_clause: Option<&GrammarASTNode>,
    agg_specs: &[(String, String)],
) -> Result<Vec<HashMap<String, SqlValue>>, ExecutionError> {
    let group_keys: Vec<String> = group_clause
        .map(extract_group_keys)
        .unwrap_or_default();

    // Partition into groups using a Vec of (key, rows) to preserve order.
    let mut groups: Vec<(Vec<SqlValue>, Vec<HashMap<String, SqlValue>>)> = Vec::new();

    for row in rows {
        let key: Vec<SqlValue> = group_keys.iter()
            .map(|k| row.get(k).cloned().unwrap_or(None))
            .collect();
        if let Some(group) = groups.iter_mut().find(|(k, _)| k == &key) {
            group.1.push(row);
        } else {
            groups.push((key.clone(), vec![row]));
        }
    }

    let mut result = Vec::new();
    for (key_values, group_rows) in groups {
        let mut rep_row = group_rows[0].clone();
        for (k, v) in group_keys.iter().zip(key_values.iter()) {
            rep_row.insert(k.clone(), v.clone());
        }
        if !agg_specs.is_empty() {
            let agg_result = compute_aggregates(&group_rows, agg_specs);
            rep_row.extend(agg_result);
        }

        if let Some(hc) = having_clause {
            if let Some(expr) = find_expr_in_clause(hc) {
                let val = eval_expr(expr, &rep_row)?;
                if !is_truthy(&val) { continue; }
            }
        }

        result.push(rep_row);
    }

    Ok(result)
}

fn extract_group_keys(group_clause: &GrammarASTNode) -> Vec<String> {
    group_clause.children.iter()
        .filter(|c| matches!(c, ASTNodeOrToken::Node(_)))
        .map(|c| node_text(c).trim().to_string())
        .collect()
}

fn collect_agg_specs_node(node: &GrammarASTNode, specs: &mut Vec<(String, String)>) {
    if node.rule_name == "function_call" {
        if let Some(ASTNodeOrToken::Token(name_tok)) = node.children.first() {
            let func_name = name_tok.value.to_uppercase();
            if ["COUNT", "SUM", "AVG", "MIN", "MAX"].contains(&func_name.as_str()) {
                let arg_parts: String = node.children[2..node.children.len().saturating_sub(1)]
                    .iter()
                    .filter_map(|c| match c {
                        ASTNodeOrToken::Token(tok) if tok.value != "," => Some(tok.value.clone()),
                        ASTNodeOrToken::Node(n) => Some(node_text_from_ast(n)),
                        _ => None,
                    })
                    .collect();
                let pair = (func_name, arg_parts);
                if !specs.contains(&pair) {
                    specs.push(pair);
                }
            }
        }
    }
    for child in &node.children {
        if let ASTNodeOrToken::Node(n) = child {
            collect_agg_specs_node(n, specs);
        }
    }
}

// ---------------------------------------------------------------------------
// Phase 6: SELECT projection
// ---------------------------------------------------------------------------

fn apply_select(
    select_list: Option<&GrammarASTNode>,
    rows: Vec<HashMap<String, SqlValue>>,
    primary_table: &str,
    primary_alias: &str,
    source: &dyn DataSource,
) -> Result<(Vec<String>, Vec<HashMap<String, SqlValue>>), ExecutionError> {
    let Some(sl) = select_list else { return Ok((vec![], rows)); };

    if is_star_select(sl) {
        if rows.is_empty() {
            let cols = source.schema(if primary_alias.is_empty() { primary_table } else { primary_alias })?;
            return Ok((cols, vec![]));
        }
        let cols = source.schema(primary_table)?;
        let projected: Vec<HashMap<String, SqlValue>> = rows.iter()
            .map(|row| cols.iter().map(|c| (c.clone(), row.get(c).cloned().unwrap_or(None))).collect())
            .collect();
        return Ok((cols, projected));
    }

    let items = find_children(sl, "select_item");
    let items: Vec<&GrammarASTNode> = if items.is_empty() { vec![sl] } else { items };

    let columns: Vec<String> = items.iter().map(|item| extract_item_name(item).0).collect();

    let projected: Result<Vec<_>, _> = rows.iter().map(|row| {
        let mut proj = HashMap::new();
        for item in &items {
            let (col_name, expr_opt) = extract_item_name(item);
            let val = if let Some(expr) = expr_opt {
                match eval_expr(expr, row) {
                    Ok(v) => v,
                    Err(ExecutionError::ColumnNotFound(_)) => None,
                    Err(e) => return Err(e),
                }
            } else {
                None
            };
            proj.insert(col_name, val);
        }
        Ok(proj)
    }).collect();

    Ok((columns, projected?))
}

fn is_star_select(sl: &GrammarASTNode) -> bool {
    sl.children.iter().any(|c| {
        if let ASTNodeOrToken::Token(tok) = c { tok.value == "*" }
        else if let ASTNodeOrToken::Node(n) = c {
            n.children.iter().any(|gc| {
                if let ASTNodeOrToken::Token(tok) = gc { tok.value == "*" } else { false }
            })
        }
        else { false }
    })
}

fn extract_item_name<'a>(item: &'a GrammarASTNode) -> (String, Option<&'a ASTNodeOrToken>) {
    let mut alias: Option<String> = None;
    let mut expr_node: Option<&'a ASTNodeOrToken> = None;

    let children = &item.children;
    let mut i = 0;
    while i < children.len() {
        let child = &children[i];
        if keyword_value_of(child) == "AS" {
            if let Some(next) = children.get(i + 1) {
                alias = Some(token_value_of(next));
            }
            i += 2;
            continue;
        }
        if expr_node.is_none() && keyword_value_of(child) != "AS" {
            expr_node = Some(child);
        }
        i += 1;
    }

    let name = alias.unwrap_or_else(|| {
        expr_node.map(infer_col_name).unwrap_or_else(|| "?".to_string())
    });
    (name, expr_node)
}

fn infer_col_name(node: &ASTNodeOrToken) -> String {
    match node {
        ASTNodeOrToken::Token(tok) => tok.value.clone(),
        ASTNodeOrToken::Node(n) => {
            if let Some(name) = infer_wrapped_function_name(n) {
                return name;
            }
            if n.rule_name == "column_ref" {
                // Use the last NAME token
                if let Some(last_token) = n.children.iter().rev().find_map(|c| {
                    if let ASTNodeOrToken::Token(tok) = c { Some(tok) } else { None }
                }) {
                    return last_token.value.clone();
                }
            }
            if n.rule_name == "function_call" {
                // Normalize function name to uppercase so that SUM(salary),
                // sum(salary), and Sum(salary) all produce "SUM(salary)".
                // The SQL lexer lowercases all tokens in case-insensitive mode
                // (because @case_insensitive true lowercases the source text
                // before pattern matching), so without this step the output
                // column name would be "sum(salary)" on all platforms.
                if let Some(ASTNodeOrToken::Token(func_tok)) = n.children.first() {
                    let func_upper = func_tok.value.to_uppercase();
                    let rest: String = n.children[1..].iter().map(node_text).collect();
                    return format!("{func_upper}{rest}");
                }
            }
            node_text_from_ast(n)
        }
    }
}

fn infer_wrapped_function_name(node: &GrammarASTNode) -> Option<String> {
    if node.rule_name == "function_call" {
        if let Some(ASTNodeOrToken::Token(func_tok)) = node.children.first() {
            let func_upper = func_tok.value.to_uppercase();
            let rest: String = node.children[1..].iter().map(node_text).collect();
            return Some(format!("{func_upper}{rest}"));
        }
    }

    if node.children.len() == 1 {
        if let ASTNodeOrToken::Node(child) = &node.children[0] {
            return infer_wrapped_function_name(child);
        }
    }

    None
}

// ---------------------------------------------------------------------------
// Phase 7: DISTINCT
// ---------------------------------------------------------------------------

fn apply_distinct(rows: Vec<HashMap<String, SqlValue>>) -> Vec<HashMap<String, SqlValue>> {
    let mut seen: Vec<Vec<SqlValue>> = Vec::new();
    let mut result = Vec::new();
    for row in rows {
        // Collect values in sorted key order for consistent comparison
        let mut keys: Vec<&String> = row.keys().collect();
        keys.sort();
        let key: Vec<SqlValue> = keys.iter().map(|k| row[*k].clone()).collect();
        if !seen.contains(&key) {
            seen.push(key);
            result.push(row);
        }
    }
    result
}

// ---------------------------------------------------------------------------
// Phase 8: ORDER BY
// ---------------------------------------------------------------------------

fn apply_order_by(
    mut rows: Vec<HashMap<String, SqlValue>>,
    order_clause: &GrammarASTNode,
) -> Result<Vec<HashMap<String, SqlValue>>, ExecutionError> {
    let order_items = find_children(order_clause, "order_item");
    if order_items.is_empty() { return Ok(rows); }

    // Apply sorts from last to first (stable sort).
    for item in order_items.iter().rev() {
        let (expr, ascending) = extract_order_item(item);
        rows.sort_by(|a, b| {
            let va = eval_expr(expr, a).unwrap_or(None);
            let vb = eval_expr(expr, b).unwrap_or(None);
            let ord = compare_sql_values(&va, &vb);
            if ascending { ord } else { ord.reverse() }
        });
    }

    Ok(rows)
}

fn extract_order_item(item: &GrammarASTNode) -> (&ASTNodeOrToken, bool) {
    let mut ascending = true;
    let expr = &item.children[0];
    for child in &item.children[1..] {
        let kw = keyword_value_of(child);
        if kw == "DESC" { ascending = false; }
        if kw == "ASC" { ascending = true; }
    }
    (expr, ascending)
}

fn compare_sql_values(a: &SqlValue, b: &SqlValue) -> std::cmp::Ordering {
    match (a, b) {
        (None, None) => std::cmp::Ordering::Equal,
        (None, _) => std::cmp::Ordering::Greater, // NULL sorts last
        (_, None) => std::cmp::Ordering::Less,
        (Some(SqlPrimitive::Int(x)), Some(SqlPrimitive::Int(y))) => x.cmp(y),
        (Some(SqlPrimitive::Float(x)), Some(SqlPrimitive::Float(y))) => {
            x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal)
        }
        (Some(SqlPrimitive::Int(x)), Some(SqlPrimitive::Float(y))) => {
            (*x as f64).partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal)
        }
        (Some(SqlPrimitive::Float(x)), Some(SqlPrimitive::Int(y))) => {
            x.partial_cmp(&(*y as f64)).unwrap_or(std::cmp::Ordering::Equal)
        }
        (Some(SqlPrimitive::Text(x)), Some(SqlPrimitive::Text(y))) => {
            x.to_lowercase().cmp(&y.to_lowercase())
        }
        (Some(SqlPrimitive::Bool(x)), Some(SqlPrimitive::Bool(y))) => x.cmp(y),
        _ => std::cmp::Ordering::Equal,
    }
}

// ---------------------------------------------------------------------------
// Phase 9: LIMIT
// ---------------------------------------------------------------------------

fn apply_limit(
    rows: Vec<HashMap<String, SqlValue>>,
    limit_clause: &GrammarASTNode,
) -> Vec<HashMap<String, SqlValue>> {
    let mut limit: Option<usize> = None;
    let mut offset: usize = 0;

    let children = &limit_clause.children;
    let mut i = 0;
    while i < children.len() {
        let kw = keyword_value_of(&children[i]);
        if kw == "LIMIT" {
            i += 1;
            if i < children.len() {
                limit = token_value_of(&children[i]).parse().ok();
            }
        } else if kw == "OFFSET" {
            i += 1;
            if i < children.len() {
                offset = token_value_of(&children[i]).parse().unwrap_or(0);
            }
        }
        i += 1;
    }

    let start = offset.min(rows.len());
    match limit {
        Some(n) => rows.into_iter().skip(start).take(n).collect(),
        None => rows.into_iter().skip(start).collect(),
    }
}

// ---------------------------------------------------------------------------
// DISTINCT qualifier
// ---------------------------------------------------------------------------

fn has_distinct_qualifier(stmt: &GrammarASTNode) -> bool {
    stmt.children.iter().any(|c| keyword_value_of(c) == "DISTINCT")
}

// ---------------------------------------------------------------------------
// Generic AST helpers
// ---------------------------------------------------------------------------

fn find_child<'a>(node: &'a GrammarASTNode, rule_name: &str) -> Option<&'a GrammarASTNode> {
    node.children.iter().find_map(|c| {
        if let ASTNodeOrToken::Node(n) = c {
            if n.rule_name == rule_name { Some(n) } else { None }
        } else {
            None
        }
    })
}

fn find_children<'a>(node: &'a GrammarASTNode, rule_name: &str) -> Vec<&'a GrammarASTNode> {
    node.children.iter().filter_map(|c| {
        if let ASTNodeOrToken::Node(n) = c {
            if n.rule_name == rule_name { Some(n) } else { None }
        } else {
            None
        }
    }).collect()
}

fn find_expr_in_clause(clause: &GrammarASTNode) -> Option<&ASTNodeOrToken> {
    for child in &clause.children {
        let kw = keyword_value_of(child);
        if ["WHERE", "HAVING", "ON"].contains(&kw.as_str()) { continue; }
        return Some(child);
    }
    None
}

fn first_token_value(node: &GrammarASTNode) -> String {
    node.children.iter().find_map(|c| {
        if let ASTNodeOrToken::Token(tok) = c { Some(tok.value.clone()) } else { None }
    }).unwrap_or_default()
}

fn keyword_value_of(node: &ASTNodeOrToken) -> String {
    match node {
        ASTNodeOrToken::Token(tok) => tok.value.to_uppercase(),
        ASTNodeOrToken::Node(n) => {
            if let Some(first) = n.children.first() { keyword_value_of(first) }
            else { String::new() }
        }
    }
}

fn token_value_of(node: &ASTNodeOrToken) -> String {
    match node {
        ASTNodeOrToken::Token(tok) => tok.value.clone(),
        ASTNodeOrToken::Node(n) => {
            if let Some(first) = n.children.first() { token_value_of(first) }
            else { String::new() }
        }
    }
}

fn node_text_from_ast(node: &GrammarASTNode) -> String {
    node.children.iter().map(node_text).collect()
}

fn is_truthy(val: &SqlValue) -> bool {
    matches!(val, Some(SqlPrimitive::Bool(true)))
        || matches!(val, Some(SqlPrimitive::Int(i)) if *i != 0)
        || matches!(val, Some(SqlPrimitive::Float(f)) if *f != 0.0)
        || matches!(val, Some(SqlPrimitive::Text(s)) if !s.is_empty())
}
