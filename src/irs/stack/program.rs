use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use petgraph::Direction;
use petgraph::stable_graph::StableGraph;
use crate::logger::LogEvent;
use crate::util::{FilePos, Positioned};
use crate::irs::stack::procedure::{ProcedureKind, StackProcedure};
use crate::irs::stack::statement::StackStatement;

pub struct StackProgram {
    /// This contains all procedures that make up this program, including interrupt handlers
    procedures: Vec<Positioned<StackProcedure>>,
    entry_id: Option<usize>,
    nmi_id: Option<usize>,
    hwi_id: Option<usize>,
    swi_ids: [Option<usize>; 256],
    flow_graph: StableGraph<usize, ()>,
}
impl Display for StackProgram {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}
impl StackProgram {
    pub fn empty() -> StackProgram {
        StackProgram {
            procedures: Vec::new(),
            entry_id: None,
            nmi_id: None,
            hwi_id: None,
            swi_ids: [None; 256],
            flow_graph: StableGraph::new(),
        }
    }

    pub fn add_procedure(&mut self, mut procedure: Positioned<StackProcedure>) -> Result<usize, LogEvent> {
        procedure.set_id(self.procedures.len());

        match procedure.kind() {
            ProcedureKind::Main => {
                if self.entry_id.is_some() {
                    return Err(LogEvent::error("illegal redefinition of main procedure", procedure.pos().clone()));
                }
                self.entry_id = Some(procedure.id());
            }
            ProcedureKind::Default => {}
            ProcedureKind::Swi(id) => {
                if self.swi_ids[*id as usize].is_some() {
                    return Err(LogEvent::error(&format!("illegal redefinition of swi handler {}", id), procedure.pos().clone()));
                }
                self.swi_ids[*id as usize] = Some(procedure.id());
            }
            ProcedureKind::Nmi => {
                if self.nmi_id.is_some() {
                    return Err(LogEvent::error("illegal redefinition of nmi handler", procedure.pos().clone()));
                }
                self.nmi_id = Some(procedure.id());
            }
            ProcedureKind::Hwi => {
                if self.hwi_id.is_some() {
                    return Err(LogEvent::error("illegal redefinition of hwi handler", procedure.pos().clone()));
                }
                self.hwi_id = Some(procedure.id());
            }
        }

        self.procedures.push(procedure);

        Ok(self.procedures.len() - 1)
    }

    pub fn get_entry_id(&self) -> Result<usize, LogEvent> {
        self.entry_id.ok_or_else(|| { LogEvent::error("no entry point in program", FilePos::default()) })
    }

    pub fn get_entry_procedure(&self) -> Option<&Positioned<StackProcedure>> {
        if let Some(entry_id) = self.entry_id {
            return self.procedures.get(entry_id)
        }
        None
    }

    pub fn get_procedures(&self) -> &[Positioned<StackProcedure>] {
        &self.procedures
    }

    pub fn get_procedures_mut(&mut self) -> &mut [Positioned<StackProcedure>] {
        &mut self.procedures
    }

    pub fn reachable_procedure_is(&self) -> Vec<usize> {
        let mut reachable = vec![self.entry_id.expect(
            "program has no entry procedure, did you remember to build the flow graph?"
        )];
        if let Some(hwi_id) = self.hwi_id {
            reachable.push(hwi_id);
        }
        if let Some(nmi_id) = self.nmi_id {
            reachable.push(nmi_id);
        }
        for maybe in self.swi_ids {
            if let Some(swi_id) = maybe {
                reachable.push(swi_id);
            }
        }
        for node_idx in self.flow_graph.node_indices() {
            let neighbors_incoming = self.flow_graph.neighbors_directed(node_idx, Direction::Incoming).collect::<Vec<_>>();
            // blocks with no incoming edges are not reachable
            if neighbors_incoming.len() == 0 {
                continue;
            }
            // blocks with exactly one incoming edge which is that block are not reachable
            else if neighbors_incoming.len() == 1 && neighbors_incoming[0] == node_idx {
                continue;
            }
            // any other block is reachable
            reachable.push(self.flow_graph[node_idx].clone());
        }
        reachable
    }

    /// Tries to build a control flow graph for this program
    /// If it fails, returns a LogEvent explaining why
    /// If it succeeds, flow through this program is well-formed
    pub fn build_flow_graph(&mut self) -> Result<&StableGraph<usize, ()>, LogEvent> {
        self.flow_graph.clear();
        // program must have an entry point
        self.get_entry_id()?;
        // then we can build the graph
        // remember procedure names to their ids for later
        // first insert all the nodes, and map names to indices
        let mut name_idx = HashMap::new();
        for (p_i, procedure) in self.procedures.iter().enumerate() {
            let graph_i = self.flow_graph.add_node(p_i);
            name_idx.insert(procedure.name().to_string(), graph_i); // to_string to detach keys
        }
        // then insert the edges
        // here we also fill in the indices in the call statements
        // these indices are into the procedure vector, not the procedure graph
        // having built the hashmap we avoid O(n^2)
        let mut edges = vec![];
        for p_graph_i in self.flow_graph.node_indices() {
            let p_i = self.flow_graph[p_graph_i]; // index into the procedure array
            let procedure = &mut self.procedures[p_i]; // the actual procedure
            // instead of iterating over the procedure's statements directly, we iterate over its basic blocks and then the statements within those blocks
            // this ensures that we only reach call statements which are themselves reachable from within the procedure
            for block in procedure.reachable_blocks() {
                for statement in procedure.statements_in_block_mut(block) {
                    match &mut **statement {
                        StackStatement::Call { dest_name, dest_idx, t_in: _, t_out: _ } => {
                            // grab the name out of the map, and write it into the statement
                            if let Some(&d_graph_i) = name_idx.get(dest_name.as_str()) { // index into the graph
                                let d_i = self.flow_graph[d_graph_i]; // index into the procedure array
                                *dest_idx = Some(d_i);
                                edges.push((p_graph_i, d_graph_i));
                            }
                            else {
                                return Err(LogEvent::error(&format!("call to unknown procedure \"{}\"", dest_name), statement.pos().clone()));
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        for e in edges {
            self.flow_graph.add_edge(e.0, e.1, ());
        }
        Ok(&self.flow_graph)
    }
}
