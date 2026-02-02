# Strata Killer Demos for v0.1

This document describes the two primary demonstrations that prove Strata's value proposition for the v0.1 release.

**Target User:** Automation Engineer (AI/ML Ops, SRE, DevOps, MLOps, Security)

**Core Value Propositions:**
1. **Explainability** - Every side effect is traced and auditable
2. **Safety** - Type-safe, capability-gated, no ambient authority
3. **Reproducibility** - Deterministic replay of failures from effect traces
4. **Clarity** - Explicit effects make system behavior visible

---

## Demo 1: Safe Model Deployment Script

**Use Case:** Deploy an ML model to production with full auditability

**Target Pain Points:**
- Deployment failures are hard to debug (what actually happened?)
- Scripts can accidentally hit production when they should hit staging
- No compile-time safety for configuration errors
- Hard to trace which file/API call caused a failure

### Code Example

```strata
fn deploy_model(
    model_path: String,
    endpoint: Url,
    using fs: FsCap,
    using net: NetCap,
    using time: TimeCap
) -> Result<DeploymentId, DeployError> & {FS, Net, Time} {
    // 1. Load and validate model file
    let model_data = read_file(model_path, using fs)?;
    let checksum = hash(model_data);
    log("Model loaded: {} bytes, checksum: {}", 
        len(model_data), checksum, using time);
    
    // 2. Upload to staging endpoint
    let staging_url = endpoint.with_path("/staging/upload");
    log("Uploading to staging: {}", staging_url, using time);
    let staging_id = upload_model(staging_url, model_data, using net)?;
    
    // 3. Validate deployment
    log("Validating deployment: {}", staging_id, using time);
    let validation = validate_deployment(staging_id, using net)?;
    
    if !validation.passed {
        return Err(DeployError::ValidationFailed(validation.errors));
    }
    
    // 4. Promote to production
    log("Promoting {} to production", staging_id, using time);
    let prod_id = promote_to_production(staging_id, using net)?;
    
    log("Deployment complete: {}", prod_id, using time);
    Ok(prod_id)
}
```

### Effect Trace Output

```json
{
  "function": "deploy_model",
  "start_time": "2026-01-31T10:30:00Z",
  "effects": [
    {
      "timestamp": "2026-01-31T10:30:00.100Z",
      "effect": "FS.Read",
      "path": "/models/sentiment-v2.pkl",
      "bytes": 1048576,
      "checksum": "abc123..."
    },
    {
      "timestamp": "2026-01-31T10:30:00.250Z",
      "effect": "Time.Log",
      "message": "Model loaded: 1048576 bytes, checksum: abc123..."
    },
    {
      "timestamp": "2026-01-31T10:30:00.300Z",
      "effect": "Net.Post",
      "url": "https://ml-api.company.com/staging/upload",
      "request_bytes": 1048576,
      "response_status": 200,
      "response_body": "{\"staging_id\": \"stg-789\"}"
    },
    {
      "timestamp": "2026-01-31T10:30:01.500Z",
      "effect": "Net.Get",
      "url": "https://ml-api.company.com/validate/stg-789",
      "response_status": 200,
      "response_body": "{\"passed\": true, \"errors\": []}"
    },
    {
      "timestamp": "2026-01-31T10:30:01.600Z",
      "effect": "Net.Post",
      "url": "https://ml-api.company.com/promote/stg-789",
      "response_status": 200,
      "response_body": "{\"prod_id\": \"prod-456\"}"
    }
  ],
  "result": "Ok(\"prod-456\")",
  "duration_ms": 1500
}
```

### Demo Narrative

**Setup:** Show traditional Python deployment script with:
- Hidden side effects
- Runtime type errors
- Hard-to-debug failures
- No audit trail

**Strata version:**
1. Show the code with explicit effects
2. Show compile-time type checking catching config errors
3. Show capability gating preventing accidental prod access
4. Run the deployment successfully
5. Show the effect trace (full audit trail)
6. Simulate a failure (validation fails)
7. Use replay to reproduce the exact failure state
8. Fix the issue and re-run

**Wow moments:**
- "Every file read, HTTP call, and log is in the trace"
- "Can't accidentally hit prod without the right capability"
- "Type checker catches config errors before running"
- "Replay shows exactly what happened during the failure"

---

## Demo 2: AI-Powered Incident Response Workflow

**Use Case:** Automated incident response with traceable AI decision-making

**Target Pain Points:**
- AI agent actions are opaque (what did it decide and why?)
- Multi-step workflows fail midway and it's hard to know where/why
- AI calls can rack up costs with no visibility
- Hard to audit whether AI agent followed safe procedures

### Code Example

```strata
fn incident_response_workflow(
    alert: Alert,
    using net: NetCap,
    using fs: FsCap,
    using ai: AiCap,
    using time: TimeCap
) -> Result<Resolution, IncidentError> & {Net, FS, AI, Time} {
    
    // Step 1: Gather context
    log("=== INCIDENT RESPONSE: {} ===", alert.id, using time);
    log("Service: {}, Severity: {}", alert.service, alert.severity, using time);
    
    let logs = fetch_service_logs(
        alert.service,
        alert.timerange,
        using net
    )?;
    log("Fetched {} log lines", len(logs), using time);
    
    // Step 2: AI analysis
    log("Requesting AI analysis", using time);
    let analysis = ai_analyze(
        prompt: "Analyze these service logs and identify the root cause of errors",
        context: logs,
        model: "claude-sonnet-4",
        using ai
    )?;
    log("AI analysis complete: {}", analysis.summary, using time);
    
    // Step 3: Generate remediation steps
    log("Generating remediation steps", using time);
    let remediation = ai_generate_steps(
        prompt: "Generate safe remediation steps for: {}",
        context: analysis.root_cause,
        model: "claude-sonnet-4",
        using ai
    )?;
    log("Remediation plan: {} steps", len(remediation.steps), using time);
    
    // Step 4: Execute safe actions (with human approval check)
    let execution_result = if remediation.requires_approval {
        log("Remediation requires approval, skipping auto-execution", using time);
        ExecutionResult::RequiresApproval
    } else {
        log("Auto-executing safe remediation", using time);
        execute_safe_remediation(remediation.steps, using net)?
    };
    
    // Step 5: Document incident
    let report = IncidentReport {
        alert: alert,
        analysis: analysis,
        remediation: remediation,
        execution: execution_result,
        timestamp: now(using time),
    };
    
    let report_path = "/incidents/{}.md";
    write_file(report_path, format_report(report), using fs)?;
    log("Incident report saved: {}", report_path, using time);
    
    Ok(Resolution { report: report_path, status: execution_result })
}
```

### Effect Trace Output (Abbreviated)

```json
{
  "workflow": "incident_response_workflow",
  "alert_id": "inc-2026-01-31-001",
  "start_time": "2026-01-31T14:22:00Z",
  "effects": [
    {
      "step": 1,
      "timestamp": "2026-01-31T14:22:00.100Z",
      "effect": "Time.Log",
      "message": "=== INCIDENT RESPONSE: inc-2026-01-31-001 ==="
    },
    {
      "step": 1,
      "timestamp": "2026-01-31T14:22:00.200Z",
      "effect": "Net.Get",
      "url": "https://logs.company.com/api/search?service=payment-api&time=...",
      "response_bytes": 52000,
      "log_lines": 1247
    },
    {
      "step": 2,
      "timestamp": "2026-01-31T14:22:01.100Z",
      "effect": "AI.Analyze",
      "model": "claude-sonnet-4",
      "prompt_tokens": 5247,
      "completion_tokens": 823,
      "reasoning_trace": {
        "root_cause": "Database connection pool exhausted",
        "evidence": "Logs show 50 consecutive timeout errors on DB queries",
        "confidence": 0.92
      }
    },
    {
      "step": 3,
      "timestamp": "2026-01-31T14:22:03.500Z",
      "effect": "AI.Generate",
      "model": "claude-sonnet-4",
      "prompt_tokens": 1200,
      "completion_tokens": 450,
      "generated_steps": [
        "1. Restart payment-api service to reset connection pool",
        "2. Increase connection pool max from 10 to 20",
        "3. Monitor for 5 minutes to verify resolution"
      ],
      "safety_assessment": "safe_for_auto_execution"
    },
    {
      "step": 4,
      "timestamp": "2026-01-31T14:22:04.000Z",
      "effect": "Net.Post",
      "url": "https://api.company.com/services/payment-api/restart",
      "response_status": 200
    },
    {
      "step": 5,
      "timestamp": "2026-01-31T14:22:05.000Z",
      "effect": "FS.Write",
      "path": "/incidents/inc-2026-01-31-001.md",
      "bytes": 4096
    }
  ],
  "result": "Ok",
  "total_duration_ms": 5200,
  "ai_cost_estimate": "$0.04"
}
```

### Demo Narrative

**Setup:** Explain the scenario:
- Production service is failing
- Traditional approach: Manual log diving, unclear AI reasoning, hard to audit
- Strata approach: Automated workflow with full traceability

**Demonstration:**
1. Trigger alert (simulated service failure)
2. Watch workflow execute with real-time log output
3. Show AI making decisions (visible in trace)
4. Show remediation being auto-executed (safely)
5. Show generated incident report
6. **Key moment:** Show the complete effect trace with AI reasoning
7. Replay the workflow to verify determinism
8. Show how you can audit: "Did AI follow safe procedures?"

**Wow moments:**
- "Every AI decision is traced with reasoning"
- "You can see exactly what the AI concluded and why"
- "The workflow is reproducible from the trace"
- "AI calls are capability-gated (can't call AI without explicit permission)"
- "Cost tracking built-in (every AI call shows token usage)"
- "Safety checking: AI-generated steps are validated before execution"

### Why This Demo Matters (2026 Context)

**Timing:** AI agents for ops/automation are HOT right now, but:
- People don't trust them (opaque decision-making)
- Hard to audit (did the agent do the right thing?)
- Runaway costs (AI calls without visibility)
- Safety concerns (what if agent makes destructive changes?)

**Strata solves all of these:**
- ✅ Transparent: Every AI call is traced with reasoning
- ✅ Auditable: Effect trace shows complete decision chain
- ✅ Cost-controlled: Token usage visible in trace
- ✅ Safe: AI capabilities are explicitly granted, not ambient

**Market positioning:** "Strata is how you build trustworthy AI agents for production operations."

---

## Demo Comparison Matrix

| Feature | Traditional Script | Strata |
|---------|-------------------|--------|
| **Type Safety** | Runtime errors | Compile-time checking |
| **Effect Visibility** | Hidden side effects | Explicit in types |
| **Capability Control** | Ambient authority | Explicit capabilities |
| **Audit Trail** | Manual logging | Automatic trace |
| **Reproducibility** | Hard to replay | Deterministic replay |
| **AI Transparency** | Opaque | Full reasoning trace |
| **Error Messages** | Stack traces | Type errors with spans |

---

## Success Criteria for Demos

**Demo 1 (Deployment) succeeds if:**
- Audience says: "I wish my deploy scripts had this"
- They understand effect types from the example
- They see value in capability gating
- Effect trace is clearly useful for debugging

**Demo 2 (AI Incident Response) succeeds if:**
- Audience says: "This is how AI agents should work"
- They trust the AI more because of traceability
- They see this as production-ready, not a toy
- It generates press/discussion in AI ops communities

---

## v0.1 Requirements to Support These Demos

**Language features needed:**
- [x] Type checking with inference (Issue 004)
- [ ] Functions (Issue 005)
- [ ] Basic control flow (Issue 006)
- [ ] ADTs: `Result<T, E>`, `Option<T>`, structs (Issue 007)
- [ ] Effect syntax: `& {FS, Net, ...}` (Issue 008)
- [ ] Capabilities: `using cap: CapType` (Issue 009)
- [ ] Effect tracing runtime (Phase 4)
- [ ] Replay runner (Phase 4)

**Standard library needed:**
- `Result<T, E>` and `Option<T>` types
- String, Vec, basic collections
- File I/O: `read_file`, `write_file`
- HTTP: `http_get`, `http_post`
- AI: `ai_analyze`, `ai_generate_steps` (wrapper around OpenAI/Anthropic APIs)
- Time: `now()`, `log()`
- Utilities: `hash()`, `len()`, `format()`

**Tooling needed:**
- CLI that runs programs
- Effect trace JSON output
- Replay runner that can replay from trace
- Clear error messages

---

## Timeline for Demo Readiness

**Demo 1 (Deployment):**
- Requires: Issues 005-009 + minimal stdlib
- Estimate: Month 8-9 (after effect system complete)

**Demo 2 (AI Incident Response):**
- Requires: Same as Demo 1 + AI capability wrapper
- Estimate: Month 9-10 (polish on top of Demo 1)

**Both demos polished for v0.1 launch:**
- Month 10-12 (hardening phase)

---

## Post-Demo Evolution (v0.2+)

**Potential enhancements:**
- Add actors: Show multi-agent incident response
- Add row polymorphism: Generic workflow steps
- Add async: Show concurrent remediation steps
- Add logic engine: Show proof traces for AI reasoning

But for v0.1, the demos above are **sufficient to prove the value proposition**.
