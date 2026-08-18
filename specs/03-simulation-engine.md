# Simulation Engine Specification

## Overview

The Simulation Engine is IMPERIUM's "time machine." Before executing an intent, it runs thousands of Monte Carlo rollouts on a causal world model, showing you the probabilistic future — success rates, failure modes, costs, timelines, and counterfactuals.

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         SIMULATION ENGINE                                    │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  INPUT: IntentIR + WorldModel Snapshot                                       │
│         │                                                                    │
│         ▼                                                                    │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │ WORLD MODEL (continuously updated from execution traces)            │    │
│  │  • Code: AST + call graph + test coverage + mutation score          │    │
│  │  • Infra: K8s topology + capacity + cost + SLOs                     │    │
│  │  • Data: Schema + volume + PII tags + lineage                       │    │
│  │  • Team: Ownership + expertise + on-call + velocity                 │    │
│  │  • Traffic: Patterns + anomalies + seasonality                      │    │
│  │  • History: Incidents + deployments + config changes                │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│         │                                                                    │
│         ▼                                                                    │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │ MONTE CARLO ROLLOUTS (10,000+ parallel simulations)                 │    │
│  │  • Each rollout: executes IntentIR in simulated world               │    │
│  │  • Injects chaos: latency spikes, node failures, bugs, load spikes  │    │
│  │  • Tracks: success, latency, cost, blast radius, MTTR               │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│         │                                                                    │
│         ▼                                                                    │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │ COUNTERFACTUAL ENGINE                                               │    │
│  │  "What if we skip the staging deploy?"                              │    │
│  │  "What if traffic 3x's during migration?"                           │    │
│  │  "What if the DB migration locks for 5 min?"                        │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│         │                                                                    │
│         ▼                                                                    │
│  OUTPUT: SimulationResult                                                    │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │ ✅ SUCCESS PROBABILITY: 94%                                          │    │
│  │ ⚠️  RISK: 6% chance of 5-min auth outage (DB lock)                 │    │
│  │ 💰 COST: $230 (compute) + 2.3 eng-hours                             │    │
│  │ ⏱️  TIMELINE: 4.2 hrs median (P90: 7.1 hrs)                       │    │
│  │ 🔄 ROLLBACK: Automated, <30s (tested in 99.8% of sims)            │    │
│  │                                                                      │    │
│  │ [APPROVE]  [MODIFY CONSTRAINTS]  [VIEW FAILURE MODES]              │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

## World Model

### Representation

The world model is a **causal graph** with five node types:

```python
class WorldModel:
    nodes: Dict[EntityId, WorldNode]
    edges: List[CausalEdge]
    
class WorldNode:
    id: EntityId
    type: NodeType  # Code | Infra | Data | Team | Traffic
    properties: Dict[str, Any]
    state: NodeState  # Healthy | Degraded | Failed | Unknown
    
class CausalEdge:
    from: EntityId
    to: EntityId
    strength: float  # 0.0 - 1.0
    mechanism: CausalMechanism  # Latency | Throughput | Failure | Consistency
    latency_ms: float
```

### Node Types

| Type | Entities | Properties |
|------|----------|------------|
| **Code** | Services, libraries, functions | AST, call graph, test coverage, mutation score, ownership, complexity |
| **Infra** | Clusters, nodes, pods, databases, caches | Topology, capacity, cost, SLOs, redundancy, region |
| **Data** | Tables, schemas, streams, buckets | Volume, growth, PII tags, lineage, retention, encryption |
| **Team** | Engineers, on-call, squads | Expertise, velocity, availability, knowledge silos |
| **Traffic** | Endpoints, APIs, queues | RPS, latency distribution, error rate, seasonality, anomalies |

### Continuous Updates

```python
async def update_world_model(event: Event):
    """Update world model from execution events."""
    
    match event.payload:
        case TaskCompleted(task_id, outputs):
            # Update code node with new test coverage
            await update_code_coverage(task_id, outputs)
            
        case SimulationCompleted(success_prob, metrics):
            # Update model accuracy
            await calibrate_model(event.intent_id, success_prob, metrics)
            
        case CapabilityRegistered(capability_id):
            # Add new capability node
            await add_capability_node(capability_id)
            
        case IncidentDetected(service, severity):
            # Mark nodes as degraded
            await mark_degraded(service, severity)
```

## Monte Carlo Rollouts

### Rollout Algorithm

```python
async def run_rollout(
    intent_ir: IntentIR,
    world_model: WorldModel,
    seed: int,
    chaos_config: ChaosConfig
) -> RolloutResult:
    """Execute a single Monte Carlo rollout."""
    
    rng = Random(seed)
    simulator = RolloutSimulator(world_model, rng, chaos_config)
    
    # Initialize simulated state
    sim_state = SimulatedState(
        world=world_model.clone(),
        intent=intent_ir,
        current_task=0,
        completed_tasks=[],
        failed_tasks=[],
        total_cost=0.0,
        total_latency_ms=0,
        errors=[],
    )
    
    # Execute task DAG
    while sim_state.current_task < len(intent_ir.tasks):
        task = intent_ir.tasks[sim_state.current_task]
        
        # Check dependencies
        if not all(dep in sim_state.completed_tasks for dep in task.dependencies):
            # Wait for dependencies (simulated)
            await simulator.advance_time(rng.exponential(100))
            continue
        
        # Simulate task execution
        result = await simulator.execute_task(task, sim_state)
        
        if result.success:
            sim_state.completed_tasks.append(task.id)
            sim_state.total_cost += result.cost
            sim_state.total_latency_ms += result.duration_ms
        else:
            sim_state.failed_tasks.append(task.id)
            sim_state.errors.append(result.error)
            
            # Check retry policy
            if should_retry(task, result, sim_state):
                continue  # Retry same task
            else:
                # Trigger compensation
                await simulator.execute_compensation(task, sim_state)
                break
    
    return RolloutResult(
        success=len(sim_state.failed_tasks) == 0,
        completed_tasks=sim_state.completed_tasks,
        failed_tasks=sim_state.failed_tasks,
        total_cost=sim_state.total_cost,
        total_latency_ms=sim_state.total_latency_ms,
        errors=sim_state.errors,
        final_state=sim_state.world,
    )
```

### Chaos Injection

```python
class ChaosConfig:
    # Infrastructure failures
    node_failure_rate: float = 0.001  # per hour
    network_partition_prob: float = 0.0001
    disk_failure_rate: float = 0.00001
    
    # Performance degradation
    latency_spike_prob: float = 0.01
    latency_spike_multiplier: Tuple[float, float] = (2.0, 10.0)
    cpu_throttle_prob: float = 0.005
    
    # Load spikes
    traffic_spike_prob: float = 0.02
    traffic_spike_multiplier: Tuple[float, float] = (3.0, 20.0)
    
    # Bugs
    bug_trigger_prob: float = 0.001
    bug_severity: Distribution = LogNormal(0.5, 1.0)
    
    # External dependencies
    api_failure_rate: float = 0.001
    api_latency_addition: Distribution = Exponential(100)

def inject_chaos(sim_state: SimulatedState, chaos: ChaosConfig, rng: Random):
    """Inject chaos events during rollout."""
    
    # Node failure
    if rng.random() < chaos.node_failure_rate * sim_state.time_delta_hours:
        node = rng.choice(list(sim_state.world.infra_nodes))
        sim_state.world.fail_node(node)
    
    # Latency spike
    if rng.random() < chaos.latency_spike_prob:
        service = rng.choice(list(sim_state.world.code_nodes))
        multiplier = rng.uniform(*chaos.latency_spike_multiplier)
        sim_state.world.add_latency(service, multiplier)
    
    # Traffic spike
    if rng.random() < chaos.traffic_spike_prob:
        endpoint = rng.choice(list(sim_state.world.traffic_nodes))
        multiplier = rng.uniform(*chaos.traffic_spike_multiplier)
        sim_state.world.add_load(endpoint, multiplier)
```

## Counterfactual Engine

### Query Language

```python
class CounterfactualQuery:
    """Natural language → structured counterfactual."""
    
    def parse(query: str) -> Counterfactual:
        # "What if traffic 3x during migration?"
        # → Counterfactual(intervention=TrafficMultiplier(3.0), 
        #                 condition=DuringPhase("migration"))
        
        # "What if we skip staging?"
        # → Counterfactual(intervention=SkipPhase("staging"))
        
        # "What if DB locks for 5 min?"
        # → Counterfactual(intervention=NodeFailure("db", duration=300))
```

### Evaluation

```python
async def evaluate_counterfactual(
    base_simulation: SimulationResult,
    counterfactual: Counterfactual,
    n_rollouts: int = 1000
) -> CounterfactualResult:
    """Evaluate counterfactual by re-running with intervention."""
    
    # Apply intervention to world model
    modified_model = apply_intervention(base_simulation.world_model, counterfactual)
    
    # Run rollouts with modified model
    results = await run_rollouts(
        intent_ir=base_simulation.intent_ir,
        world_model=modified_model,
        n_rollouts=n_rollouts,
    )
    
    # Compare distributions
    return CounterfactualResult(
        base=base_simulation.summary,
        counterfactual=summary(results),
        diff=compare_distributions(base_simulation.summary, summary(results)),
        statistical_significance=bayesian_test(base_simulation.rollouts, results),
    )
```

## Simulation Result

```json
{
  "intent_id": "IntentId",
  "timestamp": "DateTime<Utc>",
  "rollouts": 50000,
  "success_probability": 0.942,
  "success_ci": [0.938, 0.946],
  
  "risk_factors": [
    {"factor": "DB lock during migration", "probability": 0.06, "severity": "high", "impact_minutes": 5},
    {"factor": "Cache stampede on cutover", "probability": 0.02, "severity": "medium", "impact_minutes": 2}
  ],
  
  "cost_estimate": {
    "mean": 230.0,
    "std": 45.0,
    "p50": 210.0,
    "p90": 310.0,
    "p99": 480.0,
    "breakdown": {
      "compute": 147.0,
      "engineering_hours": 2.3,
      "cloud_services": 0.0
    }
  },
  
  "timeline": {
    "mean_hours": 4.2,
    "median_hours": 3.8,
    "p10_hours": 2.1,
    "p50_hours": 3.8,
    "p90_hours": 7.1,
    "p99_hours": 12.4
  },
  
  "rollback": {
    "automated": true,
    "mean_seconds": 28.0,
    "p99_seconds": 45.0,
    "tested_in_pct": 0.998
  },
  
  "failure_modes": [
    {"mode": "DB deadlock", "probability": 0.034, "detection": "automated", "recovery": "retry with backoff"},
    {"mode": "Migration timeout", "probability": 0.018, "detection": "health check", "recovery": "rollback"},
    {"mode": "Cache inconsistency", "probability": 0.012, "detection": "checksum", "recovery": "rebuild"}
  ],
  
  "resource_usage": {
    "peak_cpu_cores": 12.0,
    "peak_memory_gb": 8.5,
    "peak_network_mbps": 145.0,
    "peak_disk_iops": 12000
  },
  
  "rollout_details": {
    "seeds_used": [12345, 12346, ...],
    "chaos_config": {...},
    "world_model_hash": "Hash"
  }
}
```

## Interactive Preview (Frontend)

The simulation preview provides:

1. **Timeline Scrubber**: Move through simulated time
2. **Service Map**: Animated topology with health/latency
3. **Metric Charts**: Error rate, latency, cost over time
4. **Counterfactual Explorer**: "What if" queries with instant results
5. **Failure Mode Browser**: All 47 failure modes with probabilities
6. **Approval Actions**: One-click approve/modify/reject

## Accuracy Validation

```python
async def validate_simulation_accuracy():
    """Compare simulation predictions vs actual execution."""
    
    for intent in completed_intents:
        sim = await get_simulation(intent.id)
        actual = await get_execution(intent.id)
        
        # Calibration: predicted probability vs actual frequency
        predicted = sim.success_probability
        actual_success = 1.0 if actual.success else 0.0
        
        # Brier score
        brier = (predicted - actual_success) ** 2
        
        # Cost accuracy
        cost_ratio = actual.cost / sim.cost_estimate.mean
        
        # Timeline accuracy
        time_ratio = actual.duration_hours / sim.timeline.median_hours
        
        await record_calibration(
            intent_id=intent.id,
            brier_score=brier,
            cost_ratio=cost_ratio,
            time_ratio=time_ratio,
        )
```

### Target Metrics

| Metric | Target |
|--------|--------|
| Brier Score | < 0.05 |
| Cost Ratio (actual/predicted) | 0.8 - 1.2 |
| Time Ratio (actual/predicted) | 0.7 - 1.3 |
| Failure Mode Coverage | > 90% of actual failures predicted |
| Counterfactual Accuracy | > 85% directionally correct |

## Performance

| Operation | Target |
|-----------|--------|
| 10k rollouts | < 10s (CPU) |
| 100k rollouts | < 60s (CPU) |
| Counterfactual query | < 200ms (cached) |
| World model update | < 50ms |
| Preview render (10k rollouts) | 60fps |

---

*Implementation in `python/imperium_simulation/` and `frontend/packages/charts/` + `frontend/packages/graph/`.*