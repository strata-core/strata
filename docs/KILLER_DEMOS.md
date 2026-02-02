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

---

## Demo 3: Multi-LLM Software Orchestrator (The Meta Demo)

**Use Case:** Build software projects using multiple AI models (Claude, ChatGPT, Grok) with capability-constrained roles

**Target Pain Points:**
- AI agents going rogue and doing things outside their role
- No visibility into which agent made which decision
- Hard to debug multi-agent failures
- Cost tracking across multiple LLM providers
- Safety concerns about agents with too much power

**Why This Demo Is Special:**
This is the "dogfooding" demo - using Strata to build the exact type of multi-agent system that Strata was designed for. It's meta-compelling: "We built an agent orchestrator IN the agent orchestration language."

### Code Example

```strata
// Define specialized agent roles with capability constraints
capability Architect = {
  FileSystem: {Read, Write.Designs},
  Claude: {API},
  Repo: {Read, Comment}
}

capability Developer = {
  FileSystem: {Read, Write.Code},
  ChatGPT: {API},
  Repo: {Read, Write, Commit},
  GitHub: {PullRequest}
}

capability Reviewer = {
  FileSystem: {Read},
  Grok: {API},
  Repo: {Read, Comment, Approve}
}

// Multi-LLM orchestration with capability security
fn build_feature(
  spec: FeatureSpec,
  using cap: {Claude.API, ChatGPT.API, Grok.API, GitHub, FileSystem}
) & {Net, FS, GitHub, AI, Log} -> Result<PullRequest, Error> {
  
  log("=== Starting Feature Build: {} ===", spec.title, using time);
  
  // Phase 1: Design (Claude - read-only on code, can write designs)
  log("Phase 1: Architecture design (Claude)", using time);
  let design = spawn_agent::<Architect>(
    caps: Architect,
    model: "claude-sonnet-4",
    prompt: "Design architecture for: {}
Requirements: {}
Constraints: {}",
    context: spec
  ).await?;
  
  log("Design complete: {} components, {} interfaces", 
      len(design.components), len(design.interfaces), using time);
  
  // Phase 2: Implement (ChatGPT - can write code, commit)
  log("Phase 2: Implementation (ChatGPT)", using time);
  let implementation = spawn_agent::<Developer>(
    caps: Developer,
    model: "gpt-4",
    prompt: "Implement based on design: {}
Test coverage: 80%+ required
Follow project style guide",
    context: design
  ).await?;
  
  log("Implementation complete: {} files changed, {} tests added",
      len(implementation.changed_files), implementation.tests_added, using time);
  
  // Phase 3: Review (Grok - read-only, can comment)
  log("Phase 3: Code review (Grok)", using time);
  let review = spawn_agent::<Reviewer>(
    caps: Reviewer,
    model: "grok-2",
    prompt: "Review implementation for:
- Security issues
- Performance problems
- Test coverage
- Code style violations",
    context: implementation
  ).await?;
  
  log("Review complete: {} issues found, severity: {}",
      len(review.issues), review.max_severity, using time);
  
  // Phase 4: Create PR with all context
  if review.max_severity == "blocker" {
    return Err(Error::ReviewBlocked(review.issues));
  }
  
  let pr = create_pull_request(
    title: spec.title,
    design: design,
    implementation: implementation,
    review: review,
    using github
  )?;
  
  log("Pull request created: {}", pr.url, using time);
  Ok(pr)
}
```

### Effect Trace Output

```json
{
  "workflow": "build_feature",
  "feature": "Add authentication middleware",
  "start_time": "2026-02-01T10:00:00Z",
  "phases": [
    {
      "phase": "Architecture",
      "agent": "Architect",
      "model": "claude-sonnet-4",
      "timestamp": "2026-02-01T10:00:01Z",
      "effects": [
        {
          "effect": "FS.Read",
          "paths": ["/src/auth/", "/docs/architecture.md"],
          "bytes_read": 15420
        },
        {
          "effect": "AI.Generate",
          "model": "claude-sonnet-4",
          "prompt_tokens": 3200,
          "completion_tokens": 1850,
          "cost_usd": 0.03,
          "reasoning_trace": {
            "approach": "Middleware pattern with JWT validation",
            "components": ["AuthMiddleware", "TokenValidator", "UserContext"],
            "justification": "Separates auth logic from business logic, testable in isolation"
          }
        },
        {
          "effect": "FS.Write",
          "path": "/designs/auth-middleware.md",
          "bytes": 4096
        }
      ],
      "duration_ms": 8500,
      "result": "Success"
    },
    {
      "phase": "Implementation",
      "agent": "Developer",
      "model": "gpt-4",
      "timestamp": "2026-02-01T10:00:10Z",
      "effects": [
        {
          "effect": "FS.Read",
          "paths": ["/designs/auth-middleware.md", "/src/**/*.rs"],
          "bytes_read": 52000
        },
        {
          "effect": "AI.Generate",
          "model": "gpt-4",
          "prompt_tokens": 8500,
          "completion_tokens": 3200,
          "cost_usd": 0.08,
          "generated_files": [
            "/src/auth/middleware.rs",
            "/src/auth/validator.rs",
            "/tests/auth_tests.rs"
          ]
        },
        {
          "effect": "FS.Write",
          "files": 3,
          "total_bytes": 12800
        },
        {
          "effect": "Repo.Commit",
          "sha": "abc123",
          "message": "Add authentication middleware per design",
          "files_changed": 3
        }
      ],
      "duration_ms": 22000,
      "result": "Success"
    },
    {
      "phase": "Review",
      "agent": "Reviewer",
      "model": "grok-2",
      "timestamp": "2026-02-01T10:00:35Z",
      "effects": [
        {
          "effect": "FS.Read",
          "paths": ["/src/auth/**/*.rs", "/tests/auth_tests.rs"],
          "bytes_read": 12800
        },
        {
          "effect": "AI.Analyze",
          "model": "grok-2",
          "prompt_tokens": 4200,
          "completion_tokens": 900,
          "cost_usd": 0.02,
          "findings": {
            "security_issues": 0,
            "performance_concerns": 1,
            "style_violations": 2,
            "test_coverage": "87%"
          }
        },
        {
          "effect": "Repo.Comment",
          "location": "/src/auth/middleware.rs:42",
          "text": "Consider caching token validation results for performance"
        }
      ],
      "duration_ms": 12000,
      "result": "Approved with suggestions"
    },
    {
      "phase": "PR Creation",
      "timestamp": "2026-02-01T10:00:50Z",
      "effects": [
        {
          "effect": "GitHub.PullRequest",
          "number": 42,
          "url": "https://github.com/company/project/pull/42"
        }
      ],
      "duration_ms": 1500
    }
  ],
  "total_duration_ms": 44000,
  "total_cost_usd": 0.13,
  "result": "Success(PR #42)"
}
```

### Demo Narrative

**Setup:**
1. Explain the problem: "AI agents are powerful but opaque and potentially dangerous"
2. Show traditional approach: Python scripts calling LLM APIs with no safety rails
3. Demonstrate the risks: What if architect agent tries to commit code? What if developer agent tries to approve its own work?

**Strata Solution:**
1. Define three agent roles with explicit, minimal capabilities
2. Show capability constraints in code:
   - Architect CAN'T commit code (compile error if it tries)
   - Developer CAN'T approve PRs (compile error if it tries)
   - Reviewer CAN'T modify code (compile error if it tries)
3. Run the workflow end-to-end
4. Show the complete effect trace with:
   - Every AI decision with reasoning
   - Every file operation
   - Cost tracking across all models
   - Time breakdown per phase

**Wow Moments:**

1. **"The compiler prevents agent security violations"**
   - Try to make Architect commit code → Compile error
   - "This is type-safety for AI agents"

2. **"You can see exactly what each agent did and why"**
   - Show effect trace with reasoning traces
   - "Not a black box - full transparency"

3. **"Deterministic replay for debugging"**
   - Simulate implementation failure
   - Replay from Phase 2 to see exact state when it failed
   - "Debug AI decisions like you debug code"

4. **"Cost tracking built-in"**
   - Show $0.13 total cost across 3 LLM providers
   - Track token usage per agent
   - "No surprise bills"

5. **"Multi-vendor orchestration is natural"**
   - Claude for architecture (best at design)
   - ChatGPT for implementation (fast, good at code)
   - Grok for review (different perspective)
   - "Use the right model for each task"

### Why This Demo Matters (2026 Context)

**Current State of AI Agents (Feb 2026):**
- LangChain, AutoGen, CrewAI - popular but opaque
- Agents frequently exceed their intended scope
- Debugging requires reading logs and hoping
- Cost overruns common (agents making expensive calls)
- Security teams don't trust agent systems in production

**Strata's Unique Value:**
- ✅ **Transparent:** Every AI decision traced with reasoning
- ✅ **Safe:** Capability constraints enforced at compile-time
- ✅ **Debuggable:** Deterministic replay of multi-agent workflows
- ✅ **Cost-controlled:** Token usage visible in effect traces
- ✅ **Auditable:** Complete chain of custody for all agent actions

**Market Position:**
> "LangChain is for prototyping AI agents. Strata is for running them in production with confidence."

### Meta-Narrative Power

**This demo is special because:**
1. **It's self-referential:** "We used Strata to solve the exact problem Strata was designed for"
2. **It's immediately useful:** People want this tool TODAY
3. **It proves the thesis:** If Strata can orchestrate its own development, it can orchestrate anything
4. **It's demo-able:** Can show it working in real-time

**Press/Community Angle:**
> "Strata: The AI Agent Orchestrator Built With AI Agents"

This is the kind of meta-narrative that gets attention and proves technical capability simultaneously.

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
