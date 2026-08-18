# Evolution Loop Specification

## Overview

The Evolution Loop is IMPERIUM's self-improvement mechanism. It continuously detects friction in your workflows, autonomously generates patches, shadow deploys them, validates with statistical rigor, and promotes improvements — all while you sleep.

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          EVOLUTION LOOP                                      │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  1. FRICTION DETECTION (continuous)                                         │
│     • "User manually copied Jira ticket URL 47 times this week"             │
│     • "3 engineers asked 'how do I deploy to staging?' yesterday"           │
│     • "Passkey migration took 3 attempts — schema mismatch"                 │
│     • "Voice command 'summarize standup' failed 12% of time"                │
│                                                                              │
│  2. AUTONOMOUS PATCH GENERATION                                             │
│     • Synthesizes: Jira deep-link plugin, deploy FAQ bot,                   │
│       migration validator, voice model fine-tune                            │
│     • Each patch: code + tests + docs + rollback plan                       │
│                                                                              │
│  3. SHADOW DEPLOYMENT                                                       │
│     • Runs alongside production, same inputs, no side effects               │
│     • Compares: latency, accuracy, user satisfaction                        │
│     • Duration: 24-72 hrs (configurable per risk tier)                      │
│                                                                              │
│  4. A/B VALIDATION                                                          │
│     • Statistical significance testing (Bayesian)                           │
│     • Guardrails: no regression on core metrics                             │
│     • Human-in-loop for high-risk (auth, billing, security)                 │
│                                                                              │
│  5. AUTO-PROMOTE OR DISCARD                                                 │
│     • Promote: merge to registry, update manifests, notify                  │
│     • Discard: archive with learnings, feed back to synthesizer             │
│                                                                              │
│  6. KNOWLEDGE DISTILLATION                                                  │
│     • Successful patches → training data for capability synth               │
│     • Failed simulations → negative examples for sim engine                 │
│     • User corrections → intent compiler fine-tuning                        │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Friction Detection

### Sources

| Source | Signals | Examples |
|--------|---------|----------|
| **Execution Traces** | Retries, compensations, manual interventions | Task retried 3x, human approved mid-execution |
| **User Behavior** | Repeated commands, searches, corrections | `grep` for same pattern 10x, voice re-prompts |
| **Simulation** | Low success probability, high variance | 60% success, 5 failure modes > 5% |
| **Capabilities** | Frequent synthesis, test failures | Jira capability synthesized 5x this month |
| **Voice** | Wake word false positives, STT errors | "Hey OS" triggered by "hey awesome" |
| **Intent Compilation** | Low confidence, ambiguous IR | Risk score > 0.8, multiple clarifications needed |

### Detection Algorithm

```python
class FrictionDetector:
    async def detect(self, window: TimeWindow) -> List[FrictionPattern]:
        patterns = []
        
        # Pattern 1: Repeated manual steps
        manual_steps = await self.find_repeated_manual_steps(window)
        for step in manual_steps:
            if step.frequency > 5 and step.automatable:
                patterns.append(FrictionPattern(
                    type=FrictionType.MANUAL_REPETITION,
                    description=f"Manual step '{step.name}' repeated {step.frequency}x",
                    frequency=step.frequency,
                    impact=step.estimated_time_saved * step.frequency,
                    automatable=True,
                    evidence=step.evidence,
                ))
        
        # Pattern 2: High-failure capabilities
        cap_failures = await self.find_capability_failures(window)
        for cap_id, failures in cap_failures.items():
            if failures.rate > 0.1:
                patterns.append(FrictionPattern(
                    type=FrictionType.CAPABILITY_UNRELIABLE,
                    description=f"Capability {cap_id} fails {failures.rate:.0%}",
                    frequency=failures.count,
                    impact=failures.avg_latency_ms * failures.count,
                    automatable=True,
                    evidence=failures.details,
                ))
        
        # Pattern 3: Simulation-reality gap
        sim_gaps = await self.find_simulation_gaps(window)
        for gap in sim_gaps:
            if gap.brier_score > 0.1:
                patterns.append(FrictionPattern(
                    type=FrictionType.SIMULATION_INACCURATE,
                    description=f"Simulation inaccurate for {gap.intent_type}",
                    frequency=gap.count,
                    impact=gap.avg_cost_overrun,
                    automatable=True,
                    evidence=gap.details,
                ))
        
        # Pattern 4: User corrections
        corrections = await self.find_user_corrections(window)
        for correction in corrections:
            if correction.frequency > 3:
                patterns.append(FrictionPattern(
                    type=FrictionType.USER_CORRECTION,
                    description=f"User corrected '{correction.action}' {correction.frequency}x",
                    frequency=correction.frequency,
                    impact=correction.estimated_time_wasted,
                    automatable=True,
                    evidence=correction.examples,
                ))
        
        return patterns
```

## Patch Generation

### Patch Types

| Type | Trigger | Example |
|------|---------|---------|
| **Capability Synthesis** | Missing/repeated API usage | Generate `jira.deep_link()` |
| **Intent Template** | Repeated similar intents | "Deploy to X" → template |
| **Simulation Calibration** | Systematic bias | Adjust latency model |
| **Voice Model Fine-tune** | STT errors on domain terms | Fine-tune on "standup", "retro" |
| **World Model Update** | Missing causal links | Add "deploy → cache invalidation" |
| **Policy Refinement** | Repeated approvals | Auto-approve low-risk deploys |
| **Documentation** | Frequent "how do I" queries | Generate FAQ entry |

### Generation Pipeline

```python
class PatchGenerator:
    async def generate(self, pattern: FrictionPattern) -> List[Patch]:
        patches = []
        
        match pattern.type:
            case FrictionType.MANUAL_REPETITION:
                # Analyze the manual step
                step_analysis = await self.analyze_manual_step(pattern.evidence)
                
                # Generate capability
                capability = await self.synthesize_capability(
                    name=step_analysis.suggested_name,
                    operations=step_analysis.operations,
                    examples=pattern.evidence,
                )
                
                patches.append(Patch(
                    type=PatchType.CAPABILITY,
                    target=capability.id,
                    changes=capability.to_changes(),
                    tests=capability.generate_tests(),
                    docs=capability.generate_docs(),
                    rollback=RollbackPlan.uninstall(capability.id),
                ))
            
            case FrictionType.CAPABILITY_UNRELIABLE:
                # Generate fix for existing capability
                fix = await self.generate_capability_fix(
                    capability_id=pattern.evidence.capability_id,
                    failures=pattern.evidence.failures,
                )
                
                patches.append(Patch(
                    type=PatchType.CAPABILITY_FIX,
                    target=pattern.evidence.capability_id,
                    changes=fix.changes,
                    tests=fix.tests,
                    docs=fix.docs,
                    rollback=RollbackPlan.version_rollback(pattern.evidence.capability_id),
                ))
            
            case FrictionType.SIMULATION_INACCURATE:
                # Generate simulation calibration patch
                calibration = await self.generate_simulation_calibration(
                    intent_type=pattern.evidence.intent_type,
                    gaps=pattern.evidence.gaps,
                )
                
                patches.append(Patch(
                    type=PatchType.SIMULATION_CALIBRATION,
                    target="simulation_engine",
                    changes=calibration.model_updates,
                    tests=calibration.validation_tests,
                    docs=calibration.docs,
                    rollback=RollbackPlan.config_rollback("simulation"),
                ))
            
            # ... other types
        
        return patches
```

## Shadow Deployment

### Architecture

```
                    PRODUCTION TRAFFIC
                          │
                          ▼
              ┌───────────────────────┐
              │   TRAFFIC SPLITTER    │
              │   (99% / 1%)          │
              └───────────┬───────────┘
                          │
          ┌───────────────┼───────────────┐
          ▼               ▼               ▼
    ┌───────────┐   ┌───────────┐   ┌───────────┐
    │ PRODUCTION│   │  SHADOW   │   │  METRICS  │
    │  v1.2.3   │   │  v1.2.4   │   │ COLLECTOR │
    │ (stable)  │   │ (candidate)           │
    └───────────┘   └───────────┘   └───────────┘
          │               │               │
          └───────────────┼───────────────┘
                          ▼
              ┌───────────────────────┐
              │  STATISTICAL ENGINE   │
              │  (Bayesian A/B test)  │
              └───────────────────────┘
```

### Implementation

```python
class ShadowDeployer:
    async def deploy(self, patch: Patch) -> ShadowDeployment:
        # 1. Build candidate version
        candidate = await self.build_candidate(patch)
        
        # 2. Register in capability registry (shadow mode)
        await self.registry.register_shadow(candidate)
        
        # 3. Configure traffic splitter
        await self.traffic_splitter.configure(
            production="stable",
            shadow=candidate.version,
            shadow_percentage=config.shadow_traffic_pct,
        )
        
        # 4. Start metrics collection
        deployment = ShadowDeployment(
            id=uuid4(),
            patch_id=patch.id,
            candidate_version=candidate.version,
            started_at=utcnow(),
            config=config,
        )
        
        # 5. Schedule evaluation
        asyncio.create_task(self.schedule_evaluation(deployment))
        
        return deployment
```

### Evaluation

```python
async def evaluate_shadow(deployment: ShadowDeployment) -> EvaluationResult:
    # Collect metrics
    prod_metrics = await collect_metrics("production", deployment.started_at)
    shadow_metrics = await collect_metrics("shadow", deployment.started_at)
    
    # Bayesian A/B test
    results = {}
    for metric in CORE_METRICS:
        posterior = bayesian_ab_test(
            control=prod_metrics[metric],
            treatment=shadow_metrics[metric],
            prior=PRIOR[metric],
        )
        results[metric] = {
            "prob_better": posterior.prob_better,
            "expected_lift": posterior.expected_lift,
            "credible_interval": posterior.ci_95,
        }
    
    # Guardrail checks
    guardrails = check_guardrails(results)
    
    # Decision
    if all(r["prob_better"] > 0.95 for r in results.values()) and guardrails.passed:
        decision = Decision.PROMOTE
    elif any(r["prob_better"] < 0.05 for r in results.values()) or not guardrails.passed:
        decision = Decision.DISCARD
    else:
        decision = Decision.EXTEND
    
    return EvaluationResult(
        deployment_id=deployment.id,
        metrics=results,
        guardrails=guardrails,
        decision=decision,
        recommendation=generate_recommendation(results, guardrails),
    )
```

## Guardrails

| Metric | Guardrail |
|--------|-----------|
| Success Rate | ≥ baseline - 1% |
| P99 Latency | ≤ baseline × 1.1 |
| Error Rate | ≤ baseline × 1.05 |
| Cost | ≤ baseline × 1.2 |
| Security Score | ≥ baseline |
| Rollback Time | ≤ baseline × 1.5 |

## Knowledge Distillation

```python
class KnowledgeDistiller:
    async def distill(self, promoted_patches: List[Patch]):
        for patch in promoted_patches:
            # 1. Capability synthesis training data
            if patch.type == PatchType.CAPABILITY:
                await self.add_synthesis_training(
                    prompt=patch.generation_prompt,
                    completion=patch.generated_code,
                    success=True,
                )
            
            # 2. Simulation negative examples
            if patch.type == PatchType.SIMULATION_CALIBRATION:
                await self.add_simulation_negative(
                    scenario=patch.scenario,
                    prediction=patch.old_prediction,
                    actual=patch.actual_outcome,
                )
            
            # 3. Intent compiler fine-tuning
            if patch.type == PatchType.INTENT_TEMPLATE:
                await self.add_compiler_training(
                    nl=patch.template_nl,
                    ir=patch.template_ir,
                )
            
            # 4. Voice model fine-tuning
            if patch.type == PatchType.VOICE_MODEL:
                await self.queue_voice_finetune(
                    utterances=patch.correction_utterances,
                    corrections=patch.corrections,
                )
```

## Configuration

```yaml
evolution:
  enabled: true
  
  friction_detection:
    window_hours: 168  # 1 week
    min_frequency: 3
    min_impact_minutes: 10
    
  patch_generation:
    max_patches_per_cycle: 5
    risk_tiers:
      low: auto_promote_after_hours: 24
      medium: auto_promote_after_hours: 72, human_review: true
      high: auto_promote: false, human_approval: true
      
  shadow_deployment:
    traffic_percentage: 1.0
    min_duration_hours: 24
    max_duration_hours: 168
    metrics_collection_interval: 60s
    
  evaluation:
    bayesian_prior: "weakly_informative"
    significance_threshold: 0.95
    guardrails: strict
    
  distillation:
    capability_synthesis: true
    simulation_calibration: true
    compiler_finetuning: true
    voice_finetuning: true
```

## Metrics

| Metric | Target |
|--------|--------|
| Friction detection precision | > 80% |
| Patch generation success rate | > 70% |
| Shadow deployment promotion rate | > 60% |
| Mean time to promote | < 48 hrs (low risk) |
| Regression rate | < 2% |
| Knowledge distillation quality | BLEU > 0.7 |

---

*Implementation in `python/imperium_evolution/`.*