//! # oxide-health-monitor
//!
//! GPU health monitoring with ternary status.
//! Healthy(1) / Degraded(0) / Failed(-1). Auto-failover + CRDT sync.

use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Health { Healthy = 1, Degraded = 0, Failed = -1 }
impl Health { pub fn val(&self) -> i8 { *self as i8 } }

#[derive(Debug, Clone)]
pub struct GpuNode {
    pub id: String,
    pub health: Health,
    pub consecutive_heartbeats: u64,
    pub failures: u64,
    pub workload_weight: f64,
}

impl GpuNode {
    pub fn new(id: &str) -> Self {
        Self { id: id.into(), health: Health::Healthy, consecutive_heartbeats: 0, failures: 0, workload_weight: 1.0 }
    }

    pub fn heartbeat(&mut self) {
        self.consecutive_heartbeats += 1;
        if self.health == Health::Degraded && self.consecutive_heartbeats > 5 {
            self.health = Health::Healthy;
        }
    }

    pub fn fail(&mut self) {
        self.failures += 1;
        self.consecutive_heartbeats = 0;
        self.health = Health::Failed;
        self.workload_weight = 0.0;
    }

    pub fn degrade(&mut self) {
        self.health = Health::Degraded;
        self.workload_weight = 0.5;
    }

    pub fn recover(&mut self) {
        self.health = Health::Healthy;
        self.workload_weight = 1.0;
    }
}

pub struct HealthMonitor {
    nodes: HashMap<String, GpuNode>,
    failover_log: Vec<String>,
    alerts: Vec<String>,
}

impl HealthMonitor {
    pub fn new() -> Self {
        Self { nodes: HashMap::new(), failover_log: Vec::new(), alerts: Vec::new() }
    }

    pub fn add_node(&mut self, id: &str) { self.nodes.insert(id.into(), GpuNode::new(id)); }

    pub fn heartbeat(&mut self, id: &str) {
        if let Some(node) = self.nodes.get_mut(id) { node.heartbeat(); }
    }

    pub fn fail(&mut self, id: &str) {
        if let Some(node) = self.nodes.get_mut(id) {
            node.fail();
            self.redistribute(id);
            self.failover_log.push(format!("Node {} failed", id));
        }
    }

    pub fn degrade(&mut self, id: &str) {
        if let Some(node) = self.nodes.get_mut(id) { node.degrade(); }
    }

    pub fn recover(&mut self, id: &str) {
        if let Some(node) = self.nodes.get_mut(id) { node.recover(); }
    }

    fn redistribute(&mut self, failed_id: &str) {
        let healthy: Vec<String> = self.nodes.values()
            .filter(|n| n.health == Health::Healthy && n.id != failed_id)
            .map(|n| n.id.clone()).collect();
        if healthy.is_empty() {
            self.alerts.push("FLEET CRITICAL: no healthy nodes!".into());
            return;
        }
        let boost = 1.0 / healthy.len() as f64;
        for id in &healthy {
            if let Some(node) = self.nodes.get_mut(id) {
                node.workload_weight += boost;
            }
        }
    }

    pub fn fleet_health(&self) -> Health {
        let nodes: Vec<&GpuNode> = self.nodes.values().collect();
        if nodes.is_empty() { return Health::Healthy; }
        let failed = nodes.iter().filter(|n| n.health == Health::Failed).count();
        let degraded = nodes.iter().filter(|n| n.health == Health::Degraded).count();
        if failed > nodes.len() / 2 { Health::Failed }
        else if degraded + failed > nodes.len() / 2 { Health::Degraded }
        else { Health::Healthy }
    }

    pub fn healthy_nodes(&self) -> Vec<&str> {
        self.nodes.values().filter(|n| n.health == Health::Healthy).map(|n| n.id.as_str()).collect()
    }

    pub fn failed_nodes(&self) -> Vec<&str> {
        self.nodes.values().filter(|n| n.health == Health::Failed).map(|n| n.id.as_str()).collect()
    }

    pub fn crdt_merge(&mut self, other: &HealthMonitor) {
        for (id, node) in &other.nodes {
            let local = self.nodes.entry(id.clone()).or_insert_with(|| GpuNode::new(id));
            // Failed overrides everything (fail-fast)
            if node.health == Health::Failed { local.fail(); }
            else if local.health != Health::Failed && node.health == Health::Degraded { local.degrade(); }
        }
    }

    pub fn node_count(&self) -> usize { self.nodes.len() }
    pub fn alerts(&self) -> &[String] { &self.alerts }
}

impl Default for HealthMonitor {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_healthy_fleet() {
        let mut m = HealthMonitor::new();
        m.add_node("g0"); m.add_node("g1"); m.add_node("g2");
        assert_eq!(m.fleet_health(), Health::Healthy);
        assert_eq!(m.healthy_nodes().len(), 3);
    }

    #[test]
    fn test_node_failure() {
        let mut m = HealthMonitor::new();
        m.add_node("g0"); m.add_node("g1");
        m.fail("g0");
        assert_eq!(m.failed_nodes().len(), 1);
        assert_eq!(m.healthy_nodes().len(), 1);
    }

    #[test]
    fn test_failover_redistributes() {
        let mut m = HealthMonitor::new();
        m.add_node("g0"); m.add_node("g1"); m.add_node("g2");
        m.fail("g0");
        assert!(m.nodes["g1"].workload_weight > 1.0);
    }

    #[test]
    fn test_fleet_degraded() {
        let mut m = HealthMonitor::new();
        m.add_node("g0"); m.add_node("g1");
        m.degrade("g0"); m.degrade("g1");
        assert_eq!(m.fleet_health(), Health::Degraded);
    }

    #[test]
    fn test_fleet_critical() {
        let mut m = HealthMonitor::new();
        m.add_node("g0"); m.add_node("g1");
        m.fail("g0"); m.fail("g1");
        assert_eq!(m.fleet_health(), Health::Failed);
    }

    #[test]
    fn test_crdt_merge() {
        let mut m1 = HealthMonitor::new();
        m1.add_node("g0"); m1.add_node("g1");
        let mut m2 = HealthMonitor::new();
        m2.add_node("g0"); m2.add_node("g1");
        m2.fail("g0");
        m1.crdt_merge(&m2);
        assert!(m1.failed_nodes().contains(&"g0"));
    }

    #[test]
    fn test_recovery() {
        let mut m = HealthMonitor::new();
        m.add_node("g0");
        m.fail("g0");
        m.recover("g0");
        assert_eq!(m.nodes["g0"].health, Health::Healthy);
    }

    #[test]
    fn test_all_failed_alert() {
        let mut m = HealthMonitor::new();
        m.add_node("g0");
        m.fail("g0");
        assert!(m.alerts().iter().any(|a| a.contains("CRITICAL")));
    }
}
