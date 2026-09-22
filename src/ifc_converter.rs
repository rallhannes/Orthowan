use regex::Regex;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct Entity {
    pub id: String,
    pub entity_type: String,
    pub args: String,
    pub full: String,
}

pub fn convert_ifc(text: &str) -> Result<(String, usize), String> {
    let re = Regex::new(r"(?is)(#\d+)\s*=\s*([A-Z0-9_]+)\((.*?)\);").unwrap();
    let mut entities: HashMap<String, Entity> = HashMap::new();
    
    for cap in re.captures_iter(text) {
        entities.insert(
            cap[1].to_string(),
            Entity {
                id: cap[1].to_string(),
                entity_type: cap[2].to_uppercase(),
                args: cap[3].to_string(),
                full: cap[0].to_string(),
            }
        );
    }
    
    if entities.is_empty() {
        return Err("Keine IFC-Entitäten gefunden.".into());
    }

    let mut body_ctx = None;
    for (id, e) in &entities {
        if e.entity_type == "IFCGEOMETRICREPRESENTATIONSUBCONTEXT" {
            let args_upper = e.args.to_uppercase();
            if args_upper.contains("'BODY'") && args_upper.contains("'MODEL'") {
                body_ctx = Some(id.clone());
                break;
            }
        }
    }
    
    let body_ctx = body_ctx.ok_or("IFC-Body-Kontext wurde nicht gefunden.")?;

    let mut max_id = 0;
    for id in entities.keys() {
        if id.starts_with('#') {
            if let Ok(num) = id[1..].parse::<usize>() {
                if num > max_id { max_id = num; }
            }
        }
    }
    
    let mut next_id = max_id + 1;
    let mut nid = || -> String {
        let id = format!("#{}", next_id);
        next_id += 1;
        id
    };

    let mut op_host = HashMap::new();
    for (id, e) in &entities {
        if e.entity_type == "IFCRELVOIDSELEMENT" {
            let refs = extract_refs(&e.args);
            if refs.len() >= 2 {
                let op = &refs[refs.len() - 1];
                let wall = &refs[refs.len() - 2];
                op_host.insert(op.clone(), wall.clone());
            }
        }
    }
    
    if op_host.is_empty() {
        return Err("Keine Wandöffnungen (IfcRelVoidsElement) gefunden.".into());
    }

    let mut new_entities = Vec::new();
    let mut replacements = HashMap::new();
    let mut repaired = 0;

    for (op_id, wall_id) in &op_host {
        let op = if let Some(e) = entities.get(op_id) { e } else { continue };
        let wall = if let Some(e) = entities.get(wall_id) { e } else { continue };
        
        if op.entity_type != "IFCOPENINGELEMENT" && op.entity_type != "IFCOPENINGSTANDARDCASE" { continue; }
        if !wall.entity_type.starts_with("IFCWALL") { continue; }
        
        let axis = wall_axis_points(&entities, wall_id);
        if axis.is_none() { continue; }
        let (p0, p1) = axis.unwrap();
        
        let mut ax = p1[0] - p0[0];
        let mut ay = p1[1] - p0[1];
        let l = (ax * ax + ay * ay).sqrt();
        if l < 1e-9 { continue; }
        ax /= l; ay /= l;
        let nx = -ay; let ny = ax;
        
        let mut oa = split_args(&op.args);
        if oa.len() < 7 { continue; }
        
        let old_shape_id = &oa[6];
        let old_shape = if let Some(e) = entities.get(old_shape_id) { e } else { continue };
        let o_reps = extract_refs(&old_shape.args);
        if o_reps.is_empty() { continue; }
        
        let old_rep = if let Some(e) = entities.get(&o_reps[o_reps.len() - 1]) { e } else { continue };
        let solid_refs = extract_refs(&old_rep.args);
        if solid_refs.is_empty() { continue; }
        
        let old_solid = if let Some(e) = entities.get(&solid_refs[solid_refs.len() - 1]) { e } else { continue };
        if old_solid.entity_type != "IFCEXTRUDEDAREASOLID" { continue; }
        
        let sargs = split_args(&old_solid.args);
        if sargs.len() < 4 { continue; }
        
        let old_prof = if let Some(e) = entities.get(&sargs[0]) { e } else { continue };
        let pr = extract_refs(&old_prof.args);
        if pr.is_empty() { continue; }
        
        let poly = if let Some(e) = entities.get(&pr[pr.len() - 1]) { e } else { continue };
        if poly.entity_type != "IFCPOLYLINE" { continue; }
        
        let mut pts: Vec<Vec<f64>> = extract_refs(&poly.args).iter().filter_map(|id| get_point(&entities, id)).collect();
        if pts.len() > 1 && pts[0].len() == pts[pts.len()-1].len() {
            let mut diff = 0.0;
            for i in 0..pts[0].len() {
                diff += (pts[0][i] - pts[pts.len()-1][i]).abs();
            }
            if diff < 1e-9 {
                pts.pop();
            }
        }
        if pts.len() < 4 { continue; }
        
        let height = sargs[3].parse::<f64>().unwrap_or(0.0);
        if height <= 0.0 { continue; }
        
        let us: Vec<f64> = pts.iter().map(|p| p[0]*ax + p[1]*ay).collect();
        let vs: Vec<f64> = pts.iter().map(|p| p[0]*nx + p[1]*ny).collect();
        
        let u0 = us.iter().cloned().fold(f64::INFINITY, f64::min);
        let u1 = us.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let v0 = vs.iter().cloned().fold(f64::INFINITY, f64::min);
        let v1 = vs.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        
        let width = u1 - u0;
        let depth = v1 - v0;
        if width <= 1e-8 || depth <= 1e-8 { continue; }
        
        let mut zbase = 0.0;
        if let Some(old_pos) = entities.get(&sargs[1]) {
            if old_pos.entity_type == "IFCAXIS2PLACEMENT3D" {
                let rr = extract_refs(&old_pos.args);
                if !rr.is_empty() {
                    if let Some(loc) = get_point(&entities, &rr[0]) {
                        if loc.len() >= 3 {
                            zbase = loc[2];
                        }
                    }
                }
            }
        }
        
        let ox = ax * u0 + nx * v0;
        let oy = ay * u0 + ny * v0;
        
        let mut pids = Vec::new();
        for (x, y) in &[(0.0, 0.0), (width, 0.0), (width, height), (0.0, height), (0.0, 0.0)] {
            let id = nid();
            new_entities.push(format!("{} = IFCCARTESIANPOINT(({},{}));", id, fmt(*x), fmt(*y)));
            pids.push(id);
        }
        
        let poly_id = nid();
        new_entities.push(format!("{} = IFCPOLYLINE(({}));", poly_id, pids.join(",")));
        let prof_id = nid();
        new_entities.push(format!("{} = IFCARBITRARYCLOSEDPROFILEDEF(.AREA.,$,{});", prof_id, poly_id));
        let loc_id = nid();
        new_entities.push(format!("{} = IFCCARTESIANPOINT(({},{},{}));", loc_id, fmt(ox), fmt(oy), fmt(zbase)));
        let axis_id = nid();
        new_entities.push(format!("{} = IFCDIRECTION(({},{},0.));", axis_id, fmt(nx), fmt(ny)));
        let ref_id = nid();
        new_entities.push(format!("{} = IFCDIRECTION(({},{},0.));", ref_id, fmt(ax), fmt(ay)));
        let pos_id = nid();
        new_entities.push(format!("{} = IFCAXIS2PLACEMENT3D({},{},{});", pos_id, loc_id, axis_id, ref_id));
        let dir_id = nid();
        new_entities.push(format!("{} = IFCDIRECTION((0.,0.,1.));", dir_id));
        let solid_id = nid();
        new_entities.push(format!("{} = IFCEXTRUDEDAREASOLID({},{},{},{});", solid_id, prof_id, pos_id, dir_id, fmt(depth)));
        let rep_id = nid();
        new_entities.push(format!("{} = IFCSHAPEREPRESENTATION({},'Body','SweptSolid',({}));", rep_id, body_ctx, solid_id));
        let shape_id = nid();
        new_entities.push(format!("{} = IFCPRODUCTDEFINITIONSHAPE($,$,({}));", shape_id, rep_id));
        
        oa[6] = shape_id;
        replacements.insert(op_id.clone(), format!("{} = IFCOPENINGSTANDARDCASE({});", op_id, oa.join(",")));
        repaired += 1;
    }
    
    if repaired == 0 {
        return Err("Es konnte keine OrthoGraph-Öffnung repariert werden.".into());
    }
    
    let mut replaced_text = String::new();
    let mut last_end = 0;
    
    for cap in re.captures_iter(text) {
        let m = cap.get(0).unwrap();
        replaced_text.push_str(&text[last_end..m.start()]);
        
        let id = cap.get(1).unwrap().as_str();
        if let Some(rep) = replacements.get(id) {
            replaced_text.push_str(rep);
        } else {
            replaced_text.push_str(m.as_str());
        }
        last_end = m.end();
    }
    replaced_text.push_str(&text[last_end..]);
    
    let idx = replaced_text.to_uppercase().rfind("ENDSEC;").ok_or("IFC-Ende (ENDSEC) wurde nicht gefunden.")?;
    
    let out_text = format!("{}\n{}\n{}", &replaced_text[..idx], new_entities.join("\n"), &replaced_text[idx..]);
    
    Ok((out_text, repaired))
}

fn extract_refs(s: &str) -> Vec<String> {
    let re = Regex::new(r"#\d+").unwrap();
    re.find_iter(s).map(|m| m.as_str().to_string()).collect()
}

fn split_args(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut depth = 0;
    let mut in_str = false;
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let ch = chars[i];
        if ch == '\'' {
            cur.push(ch);
            if in_str && i + 1 < chars.len() && chars[i+1] == '\'' {
                cur.push('\'');
                i += 1;
            } else {
                in_str = !in_str;
            }
        } else if !in_str {
            if ch == '(' { depth += 1; cur.push(ch); }
            else if ch == ')' { depth -= 1; cur.push(ch); }
            else if ch == ',' && depth == 0 {
                out.push(cur.trim().to_string());
                cur.clear();
            } else {
                cur.push(ch);
            }
        } else {
            cur.push(ch);
        }
        i += 1;
    }
    out.push(cur.trim().to_string());
    out
}

fn get_point(entities: &HashMap<String, Entity>, id: &str) -> Option<Vec<f64>> {
    let e = entities.get(id)?;
    if e.entity_type != "IFCCARTESIANPOINT" { return None; }
    let mut s = e.args.trim().to_string();
    if s.starts_with('(') && s.ends_with(')') {
        s = s[1..s.len()-1].to_string();
    }
    Some(s.split(',').filter_map(|x| x.trim().parse::<f64>().ok()).collect())
}

fn wall_axis_points(entities: &HashMap<String, Entity>, wall_id: &str) -> Option<(Vec<f64>, Vec<f64>)> {
    let w = entities.get(wall_id)?;
    let wr = extract_refs(&w.args);
    if wr.is_empty() { return None; }
    let shape_id = &wr[wr.len() - 1];
    let shape = entities.get(shape_id)?;
    for rep_id in extract_refs(&shape.args) {
        if let Some(rep) = entities.get(&rep_id) {
            if rep.entity_type == "IFCSHAPEREPRESENTATION" && rep.args.to_uppercase().contains("'AXIS'") {
                let item_refs = extract_refs(&rep.args);
                if item_refs.is_empty() { continue; }
                if let Some(item) = entities.get(&item_refs[item_refs.len() - 1]) {
                    if item.entity_type == "IFCPOLYLINE" {
                        let pp = extract_refs(&item.args);
                        if pp.len() >= 2 {
                            let p0 = get_point(entities, &pp[0]);
                            let p1 = get_point(entities, &pp[1]);
                            if let (Some(p0), Some(p1)) = (p0, p1) {
                                return Some((p0, p1));
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

fn fmt(mut x: f64) -> String {
    if x.abs() < 1e-12 { x = 0.0; }
    let mut s = format!("{:.10}", x);
    s = s.trim_end_matches('0').to_string();
    if s.ends_with('.') { s.push('0'); }
    if !s.contains('.') { s.push_str(".0"); }
    s
}
