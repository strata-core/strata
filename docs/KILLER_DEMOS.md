# Strata Killer Demos

This document describes demonstrations that prove Strata's value proposition.

**Target User:** Automation Engineer (AI/ML Ops, SRE, DevOps, MLOps, Security)

**Core Value Propositions:**
1. **Safety** - Type-safe, capability-gated, no ambient authority
2. **Explainability** - Every side effect is traced and auditable
3. **Reproducibility** - Deterministic replay of failures from effect traces
4. **Clarity** - Explicit effects make system behavior visible

---

## Demo Strategy

**v0.1 Core Demos (3 - Ship These):**
Focused, achievable demos that prove core technical capabilities and value proposition. These will be fully working for v0.1 launch.

**v0.2 Expansion Demos (3 - Documented Now, Built Later):**
Broader use cases showing Strata's versatility across different domains (traditional ops, PII handling, cloud security). These demonstrate market breadth and are documented now but built after v0.1 proves core value.

---

# PART 1: v0.1 Core Demos (Ship These)

## Demo 1: Model Deployment with Deterministic Replay

**Use Case:** Deploy an ML model to production with full auditability and debuggability

**Target Pain Points:**
- Deployment failures are hard to debug (what actually happened?)
- Scripts can accidentally hit production when they should hit staging
- No compile-time safety for configuration errors
- Hard to trace which file/API call caused a failure
- Can't reproduce failures locally (environment changed)

**What This Demo Proves:**
✅ Effect traces provide complete audit trail  
✅ Capability security prevents unauthorized access  
✅ Deterministic replay enables local debugging of production failures  
✅ Type safety catches config errors before deployment

### Code Example

```strata
fn deploy_model(
    model_path: String,
    endpoint: Url,
    using fs: FsCap,
    using net: NetCap,
    using time: TimeCap
) -> Result<DeploymentId, DeployError> & {Fs, Net, Time} {
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
  "program": "deploy_model.strata",
  "started_at": "2026-08-15T10:30:00.000Z",
  "finished_at": "2026-08-15T10:30:01.500Z",
  "result": { "ok": "prod-456" },
  "effects": [
    {
      "seq": 0,
      "timestamp": "2026-08-15T10:30:00.100Z",
      "effect": "Fs",
      "operation": "read",
      "inputs": { "path": "/models/sentiment-v2.pkl" },
      "outputs": { "bytes": 1048576, "checksum": "abc123..." },
      "duration_ms": 50
    },
    {
      "seq": 1,
      "timestamp": "2026-08-15T10:30:00.250Z",
      "effect": "Time",
      "operation": "log",
      "inputs": { "message": "Model loaded: 1048576 bytes, checksum: abc123..." }
    },
    {
      "seq": 2,
      "timestamp": "2026-08-15T10:30:00.300Z",
      "effect": "Net",
      "operation": "post",
      "inputs": { 
        "url": "https://ml-api.company.com/staging/upload",
        "body_bytes": 1048576
      },
      "outputs": {
        "status": 200,
        "body": "{\"staging_id\": \"stg-789\"}"
      },
      "duration_ms": 800
    },
    {
      "seq": 3,
      "timestamp": "2026-08-15T10:30:01.500Z",
      "effect": "Net",
      "operation": "get",
      "inputs": { "url": "https://ml-api.company.com/validate/stg-789" },
      "outputs": {
        "status": 200,
        "body": "{\"passed\": true, \"errors\": []}"
      },
      "duration_ms": 200
    }
  ],
  "duration_ms": 1500
}
```

### Demo Narrative (7 minutes)

**Act 1: The Problem (2 minutes)**

Show traditional Python deployment script:
```python
# deploy.py - typical Python script
import requests

model = open("/models/sentiment-v2.pkl", "rb").read()
r = requests.post("https://ml-api.company.com/upload", data=model)
print(f"Deployed: {r.json()['id']}")
```

**Problems demonstrated:**
- No type safety (typo in URL? Runtime error)
- Hidden side effects (what if this also calls Datadog? Slack? You don't know)
- Can't reproduce failures (if API returns 500, can't replay locally)
- No audit trail (what EXACTLY happened?)

**Act 2: Strata Solution (3 minutes)**

1. **Show the code** with explicit effects and capabilities
2. **Compile-time safety:**
   ```bash
   $ strata build deploy.strata
   ERROR: Type mismatch at line 12
     Expected: Url
     Got: String
   ```
3. **Run successful deployment:**
   ```bash
   $ strata run deploy.strata --trace deployment.json
   Deployment complete: prod-456
   Trace saved to deployment.json
   ```
4. **Show effect trace:** Full audit trail of every operation

**Act 3: The Power of Replay (2 minutes)**

Simulate failure scenario:
```bash
# Deployment fails in production at 3 AM
$ strata run deploy.strata --trace failed.json
ERROR: ValidationFailed: Model checksum mismatch

# Next morning, debug locally with NO network access
$ strata replay failed.json
# Replays exact sequence, shows failure at line 45
# Can inspect state, add logging, fix bug
# All without touching production
```

**Wow Moments:**
1. **"Every file read, HTTP call, and log is in the trace"** - Complete audit trail
2. **"Can't accidentally hit prod without the right capability"** - Capability security
3. **"Type checker catches config errors before running"** - Compile-time safety
4. **"Replay shows exactly what happened during the failure"** - Deterministic debugging

### Requirements for Demo

**Language features:**
- [x] Type checking with inference
- [x] Functions
- [x] Control flow
- [x] ADTs (`Result<T, E>`)
- [x] Effect syntax (`& {Fs, Net, Time}`)
- [ ] Capabilities (`using cap: Cap`)
- [ ] Effect tracing runtime
- [ ] Replay runner

**Standard library:**
- `Result<T, E>`, `Option<T>`
- File I/O: `read_file`
- HTTP: `http_get`, `http_post`, `upload_model`, `validate_deployment`, `promote_to_production`
- Time: `log()`
- Utilities: `hash()`, `len()`

---

## Demo 2: Meta-Agent Orchestration with Affine Types

**Use Case:** AI agents working together safely with enforced capability constraints

**Target Pain Points:**
- AI agents are opaque (what did they decide and why?)
- Agents frequently exceed their intended scope (security risk)
- Multi-agent workflows fail midway and it's hard to know where/why
- AI agents can rack up costs with no visibility
- Hard to audit whether agents followed procedures

**What This Demo Proves:**
✅ Affine types prevent capability duplication and leaking  
✅ Compiler enforces agent capability boundaries  
✅ Every AI decision is traced with reasoning  
✅ Cost tracking built-in  
✅ Deterministic replay for multi-agent debugging

### Code Example

```strata
// Define agent roles with minimal, explicit capabilities

fn architect_agent(
    task: FeatureRequest,
    using fs_read: FsReadCap,
    using ai: AiCap
) -> Result<DesignDoc> & {Fs, Ai} {
    // CAN: Read code, generate design
    // CANNOT: Write files, commit code, create PRs
    
    let codebase = scan_directory("/src", using fs_read)?;
    let design = ai_generate(
        prompt: "Design approach for: {}",
        context: codebase,
        model: "claude-sonnet-4",
        using ai
    )?;
    
    Ok(design)
}

fn developer_agent(
    design: DesignDoc,
    using fs: FsCap,
    using git: GitCap,
    using ai: AiCap
) -> Result<CommitSha> & {Fs, Git, Ai} {
    // CAN: Read/write code, commit
    // CANNOT: Create PRs, approve, merge
    
    let code = ai_generate_code(design, using ai)?;
    write_files(code.files, using fs)?;
    
    let sha = git_commit(
        message: "Implement feature per design",
        using git
    )?;
    
    Ok(sha)
}

fn reviewer_agent(
    commit_sha: CommitSha,
    using fs_read: FsReadCap,
    using github: GitHubCap,  // AFFINE - use-at-most-once
    using ai: AiCap
) -> Result<PrUrl> & {Fs, GitHub, Ai} {
    // CAN: Read code, create PR, comment
    // CANNOT: Approve own PRs, merge, modify code
    
    let diff = read_commit_diff(commit_sha, using fs_read)?;
    let review = ai_review_code(diff, using ai)?;
    
    // Create PR - consumes GitHubCap (affine!)
    let pr_url = github_create_pr(
        title: "Feature implementation",
        body: review.summary,
        using github  // github is MOVED here
    )?;
    
    // COMPILER ERROR if you try to use 'github' again:
    // github_delete_repo(using github)?;  // ERROR: use of moved value
    
    Ok(pr_url)
}

// Orchestrator coordinates all agents
fn orchestrate_feature(
    request: FeatureRequest,
    using fs: FsCap,
    using git: GitCap,
    using github: GitHubCap,  // Affine - prevents duplication
    using ai: AiCap
) -> Result<PrUrl> & {Fs, Git, GitHub, Ai} {
    
    // Phase 1: Architecture (read-only)
    let fs_read = fs.as_read_only();  // Derive read-only cap
    let design = architect_agent(request, using fs_read, using ai)?;
    
    // Phase 2: Implementation (write access)
    let commit_sha = developer_agent(design, using fs, using git, using ai)?;
    
    // Phase 3: Review & PR creation
    // GitHubCap is CONSUMED here - prevents accidental reuse
    let pr_url = reviewer_agent(
        commit_sha,
        using fs_read,
        using github,  // Moved to reviewer_agent
        using ai
    )?;
    
    // SAFETY: Can't use 'github' here anymore (moved)
    // This prevents orchestrator from bypassing review process
    
    Ok(pr_url)
}
```

### Effect Trace Output

```json
{
  "program": "orchestrate_feature.strata",
  "feature_request": "Add user authentication",
  "started_at": "2026-08-15T14:00:00.000Z",
  "finished_at": "2026-08-15T14:01:15.000Z",
  "result": { "ok": "https://github.com/company/project/pull/42" },
  "total_cost_usd": 0.15,
  "phases": [
    {
      "phase": "Architecture",
      "agent": "architect_agent",
      "model": "claude-sonnet-4",
      "timestamp": "2026-08-15T14:00:01.000Z",
      "effects": [
        {
          "seq": 0,
          "effect": "Fs",
          "operation": "scan_directory",
          "inputs": { "path": "/src" },
          "outputs": { "files": 47, "bytes": 152000 }
        },
        {
          "seq": 1,
          "effect": "Ai",
          "operation": "generate",
          "inputs": {
            "model": "claude-sonnet-4",
            "prompt_tokens": 3500
          },
          "outputs": {
            "completion_tokens": 1200,
            "cost_usd": 0.04,
            "reasoning": {
              "approach": "JWT middleware pattern",
              "components": ["AuthMiddleware", "TokenValidator"],
              "justification": "Separates auth from business logic"
            }
          }
        }
      ],
      "duration_ms": 8500,
      "result": "Success"
    },
    {
      "phase": "Implementation",
      "agent": "developer_agent",
      "model": "gpt-4",
      "timestamp": "2026-08-15T14:00:12.000Z",
      "effects": [
        {
          "seq": 2,
          "effect": "Ai",
          "operation": "generate_code",
          "inputs": {
            "model": "gpt-4",
            "prompt_tokens": 5000
          },
          "outputs": {
            "completion_tokens": 2500,
            "cost_usd": 0.08,
            "files_generated": 3
          }
        },
        {
          "seq": 3,
          "effect": "Fs",
          "operation": "write_files",
          "inputs": { "files": 3, "total_bytes": 8500 }
        },
        {
          "seq": 4,
          "effect": "Git",
          "operation": "commit",
          "outputs": { "sha": "abc123", "message": "Implement feature per design" }
        }
      ],
      "duration_ms": 25000,
      "result": "Success"
    },
    {
      "phase": "Review",
      "agent": "reviewer_agent",
      "model": "grok-2",
      "timestamp": "2026-08-15T14:00:40.000Z",
      "effects": [
        {
          "seq": 5,
          "effect": "Fs",
          "operation": "read_diff",
          "inputs": { "commit": "abc123" },
          "outputs": { "lines_changed": 250 }
        },
        {
          "seq": 6,
          "effect": "Ai",
          "operation": "review_code",
          "inputs": {
            "model": "grok-2",
            "prompt_tokens": 3000
          },
          "outputs": {
            "completion_tokens": 800,
            "cost_usd": 0.03,
            "findings": {
              "security_issues": 0,
              "test_coverage": "89%",
              "suggestions": 2
            }
          }
        },
        {
          "seq": 7,
          "effect": "GitHub",
          "operation": "create_pr",
          "inputs": {
            "title": "Add user authentication",
            "commit": "abc123"
          },
          "outputs": {
            "pr_number": 42,
            "url": "https://github.com/company/project/pull/42"
          }
        }
      ],
      "duration_ms": 18000,
      "result": "Success"
    }
  ],
  "duration_ms": 75000
}
```

### Demo Narrative (8 minutes)

**Act 1: The Problem with Current AI Agents (2 minutes)**

Show typical AI agent framework (LangChain, AutoGen):
```python
# agents.py - typical approach
architect = Agent("claude-sonnet-4", tools=["read_file", "write_file", "git", "github"])
developer = Agent("gpt-4", tools=["read_file", "write_file", "git", "github"])
reviewer = Agent("grok-2", tools=["read_file", "write_file", "git", "github"])

# What's wrong?
# - All agents have ALL tools (no least privilege)
# - Architect could accidentally commit code
# - Developer could approve their own PR
# - Reviewer could modify code they're reviewing
# - No way to prevent these at compile time
```

**Act 2: Strata's Type-Safe Approach (3 minutes)**

1. **Show agent definitions** with explicit, minimal capabilities
2. **Demonstrate capability constraints:**
   ```strata
   fn architect_agent(
       task: FeatureRequest,
       using fs_read: FsReadCap,  // Read-only!
       using ai: AiCap
   ) -> Result<DesignDoc> & {Fs, Ai} {
       // CAN'T write files - compile error if attempted
   }
   ```

3. **Show affine types in action:**
   ```strata
   let pr_url = reviewer_agent(
       commit_sha,
       using fs_read,
       using github,  // GitHubCap moved here
       using ai
   )?;
   
   // COMPILER ERROR if orchestrator tries:
   // github_delete_repo(using github)?;
   // ERROR: use of moved value 'github'
   ```

4. **Run the workflow** end-to-end (all 3 agents)

**Act 3: The "Holy Grail" Moment (3 minutes)**

1. **Attempt to break security:**
   Try to make Architect commit code:
   ```strata
   fn architect_agent(
       task: FeatureRequest,
       using fs_read: FsReadCap,
       using ai: AiCap
   ) -> Result<DesignDoc> & {Fs, Ai} {
       let design = ai_generate(...)?;
       
       // TRY TO COMMIT (should fail)
       git_commit("Shortcut!", using git)?;  
       // COMPILER ERROR: 'git' not in scope
   ```
   **Show compile error:** Type system prevents security violation

2. **Show one-time token enforcement:**
   Try to use GitHubCap twice:
   ```strata
   let pr = github_create_pr(title, using github)?;
   
   // TRY TO DELETE REPO (should fail)
   github_delete_repo(using github)?;
   // COMPILER ERROR: use of moved value 'github'
   ```
   **This is the "Affine Types Demo"** - prevents token reuse

3. **Show complete effect trace:**
   - Every AI decision with reasoning
   - Cost tracking ($0.15 across 3 models)
   - Time breakdown per phase
   - Full audit trail

**Wow Moments:**

1. **"The compiler prevents agent security violations"**
   - Try to give Architect git access → Compile error
   - "This is type-safety for AI agents"

2. **"One-time GitHub token enforced by type system"**
   - After CreatePR, can't DeleteRepo
   - "Holy Grail of 2026 AI Safety" (external reviewer)

3. **"You can see exactly what each agent did and why"**
   - Show effect trace with AI reasoning traces
   - "Not a black box - full transparency"

4. **"Deterministic replay for debugging multi-agent workflows"**
   - Simulate implementation failure
   - Replay from Phase 2 to see exact state
   - "Debug AI decisions like you debug code"

5. **"Multi-vendor orchestration is natural"**
   - Claude for architecture (best at design)
   - ChatGPT for implementation (fast, good at code)
   - Grok for review (different perspective)
   - "Use the right model for each task"

### Why This Demo Matters (2026 Context)

**Current State (Feb 2026):**
- Industry terrified of "Agentic AI" doing destructive things
- LangChain, AutoGen, CrewAI - popular but opaque
- Agents frequently exceed intended scope
- Security teams don't trust agent systems in production

**Strata's Unique Value:**
> "Strata provides the 'Capability-Safe Sandbox for Agents' that the industry desperately needs. It's not just a better AI framework - it's a security primitive for the age of autonomous agents."

**Market Position:**
> "LangChain is for prototyping AI agents. Strata is for running them in production with confidence."

### Requirements for Demo

**Language features:**
- [x] Type checking with inference
- [x] Functions
- [x] Control flow
- [x] ADTs
- [x] Effect syntax
- [x] Capabilities (Issue 009) ✅
- [ ] Affine types (Issue 010) ← CRITICAL
- [ ] Effect tracing runtime
- [ ] Replay runner

**Standard library:**
- File I/O with read-only derivation
- Git operations
- GitHub API operations
- AI model APIs (Claude, ChatGPT, Grok)

### Authority Analysis

Because capabilities are explicit parameters and effects are tracked, you can analyze agent authority:

**Before deployment:**
```bash
# What capabilities does each agent have?
$ strata analyze blast-radius orchestrate_feature.strata

orchestrate_feature: {Fs, Git, GitHub, Ai}
├─ architect_agent: {Fs(read-only), Ai}
├─ developer_agent: {Fs, Git, Ai}
└─ reviewer_agent: {Fs(read-only), GitHub, Ai}
```

**After execution:**
```bash
# Which capabilities were actually used?
$ strata analyze trace orchestrate-run.json

reviewer_agent:
  Granted: {Fs, GitHub, Ai}
  Exercised: {Fs, GitHub, Ai}
  ✓ All capabilities used
```

This helps answer:
- What's the maximum access each agent has?
- Are any capabilities granted but unused?
- Which function granted which capability?

**Note:** These analysis tools are planned for v0.1 as basic CLI commands.

---

## Demo 3: Time-Travel Bug Hunting

**Use Case:** Debug production failures by replaying exact execution without network/filesystem access

**Target Pain Points:**
- "It works on my machine" - can't reproduce production failures
- Flaky tests caused by network variability
- Debugging requires re-running effects (slow, expensive, risky)
- Integration tests that hit real APIs (flaky, slow)

**What This Demo Proves:**
✅ Effect-purity enables deterministic replay  
✅ Heisenbug → Bohrbug transformation  
✅ Debug production failures locally without risk  
✅ Replay is a development tool, not just debugging

### Code Example

```strata
// Script that calls flaky external API
fn fetch_and_process(
    url: Url,
    using net: NetCap,
    using fs: FsCap
) -> Result<ProcessedData> & {Net, Fs} {
    // Step 1: Fetch data from API (sometimes fails)
    let response = http_get(url, using net)?;
    
    // Step 2: Process data
    let parsed = parse_json(response.body)?;
    let validated = validate_schema(parsed)?;
    
    // Step 3: Save to cache
    write_file("/cache/data.json", validated, using fs)?;
    
    // Step 4: Transform
    let transformed = transform(validated)?;
    
    Ok(transformed)
}
```

### Scenario: Production Failure

**Production run (3 AM):**
```bash
$ strata run fetch.strata --trace production-failure.json
ERROR: ParseError at line 23: Unexpected token ';' in JSON
```

**Trace captured:**
```json
{
  "program": "fetch.strata",
  "started_at": "2026-08-15T03:00:00.000Z",
  "failed_at": "2026-08-15T03:00:01.250Z",
  "result": { "err": "ParseError: Unexpected token ';'" },
  "effects": [
    {
      "seq": 0,
      "effect": "Net",
      "operation": "get",
      "inputs": { "url": "https://api.partner.com/data" },
      "outputs": {
        "status": 200,
        "body": "{\"items\": [1, 2, 3]; \"extra\": true}"  // <- Malformed JSON!
      }
    }
  ]
}
```

### Demo Narrative (5 minutes)

**Act 1: The Flaky Failure (1 minute)**

Show production failure at 3 AM:
- Script fails with ParseError
- API returned malformed JSON (rare edge case)
- Can't reproduce locally (API fixed the bug)
- No way to debug without re-running production script

**Act 2: Replay to the Rescue (2 minutes)**

```bash
# Next morning, debug locally with NO network access
$ strata replay production-failure.json

# Replay mode:
# - http_get() returns recorded response from trace
# - No actual network call made
# - Script executes deterministically
# - Fails at exact same line with exact same error

Replaying production-failure.json...
Effect[0]: Net.get → (from trace)
ERROR: ParseError at line 23: Unexpected token ';' in JSON
```

**Now you can:**
1. Inspect the malformed JSON in the trace
2. Add logging to see what parser was doing
3. Fix the bug (add error handling for malformed JSON)
4. Re-run replay to verify fix
5. Deploy fixed version with confidence

**Act 3: The Meta Power (2 minutes)**

**Replay isn't just for debugging - it's a development tool:**

```bash
# Write test against real API
$ strata run integration-test.strata --trace golden.json
All tests passed!

# Now replay becomes your integration test
$ strata replay golden.json
# Runs in milliseconds, no network calls, deterministic
```

**Wow Moments:**

1. **"Heisenbug → Bohrbug"**
   - Flaky network issue becomes reproducible bug
   - Effect-purity enables time-travel debugging

2. **"Debug production failures without touching production"**
   - Replay trace locally
   - Zero risk of making things worse

3. **"Integration tests that actually work"**
   - Capture real API responses once
   - Replay as deterministic unit tests
   - Fast, reliable, no API rate limits

4. **"Effect traces are executable"**
   - Not just logs - they're replays
   - This is the power of effect-purity

### Requirements for Demo

**Language features:**
- [x] All previous features
- [ ] Effect tracing runtime ← CRITICAL
- [ ] Replay runner ← CRITICAL

**Standard library:**
- HTTP client
- JSON parsing
- File I/O

---

# PART 2: v0.2 Expansion Demos (Documented Now, Built Later)

These demos show Strata's versatility across different domains. They're documented now to show vision and breadth, but will be built after v0.1 proves core value.

---

## Demo 4: Privacy-Preserving Data Pipeline

**Use Case:** Handle PII with compile-time data sovereignty guarantees

**Target Audience:** Data Engineers, Compliance Officers, Healthcare/Finance companies

**Target Pain Points:**
- Easy to accidentally log PII (emails, SSNs) in debugging
- Hard to ensure data never leaves sanitized state
- Compliance violations from casual mistakes
- No compile-time guarantees about data flow

**What This Demo Proves:**
✅ PII effect type prevents leakage  
✅ Data must go through scrubber before logging  
✅ Type system acts as DLP engine  
✅ Compliance by construction

### Code Example

```strata
// Define PII effect separate from Safe effect
effect Pii;   // Handling cleartext PII
effect Safe;  // Handling sanitized data

fn process_user_data(
    data: UserRecord,
    using pii: PiiCap,
    using log: LogCap
) -> Result<(), Error> & {Pii, Log} {
    
    // Raw email has Pii effect
    let raw_email = data.email;  // Type: String & {Pii}
    
    // COMPILER ERROR if you try:
    // log("Processing: {}", raw_email, using log)?;
    // ERROR: Cannot use {Pii} data in {Log} operation
    
    // Must scrub first
    let clean_email = scrub_pii(raw_email, using pii); // {Pii} → {Safe}
    
    // NOW can log (Safe data allowed in Log operations)
    log("Processing user: {}", clean_email, using log)?;  // OK
    
    // Similarly, can't send raw PII over network
    // http_post(url, raw_email, using net)?;  // ERROR
    
    // Must scrub first
    let sanitized = scrub_for_analytics(data, using pii);
    http_post(analytics_url, sanitized, using net)?;  // OK
    
    Ok(())
}
```

### Demo Narrative

**The Wow Moment:**
Show compile error when trying to log PII:
```
ERROR: Effect mismatch
  --> process.strata:12:10
   |
12 |     log("Email: {}", raw_email, using log)?;
   |              ^^^^^^^^^ value has effect {Pii}
   |
   = note: 'log' requires effect {Safe}
   = help: Use scrub_pii() to convert {Pii} → {Safe}
```

**The Value:**
> "The compiler is a Data Loss Prevention engine. You literally cannot log PII without explicitly scrubbing it first. This is compliance by construction."

**Why v0.2:** Requires finer-grained effect types (Pii as separate effect) and effect transformations (scrubbers that change effect labels).

---

## Demo 5: Blast Radius Controller (Capability Attenuation)

**Use Case:** Dynamic authority slicing for least-privilege enforcement

**Target Audience:** Cloud Platform Engineers, SREs, Security Architects

**Target Pain Points:**
- Scripts get AWS_ADMIN keys even when they only need to reboot one instance
- "All-or-nothing" credentials
- Hard to enforce least privilege programmatically
- Risk of over-privileged automation

**What This Demo Proves:**
✅ Capability slicing (fat cap → thin cap)  
✅ Affine nature prevents using high-privilege version after slicing  
✅ Least privilege enforced by type system  
✅ Dynamic authority attenuation

### Code Example

```strata
fn cleanup_temp_disks(
    using aws: AwsCap  // Full AWS access (dangerous!)
) -> Result<(), Error> & {Aws} {
    
    // Slice the capability - create restricted version
    let restricted_aws = aws
        .limit_to_region("us-east-1")
        .limit_to_service("EC2")
        .read_only();  // Type: AwsCap<Region=USEast1, Service=EC2, ReadOnly>
    
    // Original 'aws' is CONSUMED (affine types!)
    // Can't accidentally use high-privilege version later
    
    // This sub-function CAN'T delete database in us-west-2
    run_cleanup_logic(using restricted_aws)?;
    
    // COMPILER ERROR if you try:
    // delete_database("us-west-2", using aws)?;
    // ERROR: use of moved value 'aws'
    
    Ok(())
}
```

### Demo Narrative

**The Wow Moment:**

Try to use full AWS cap after slicing:
```strata
let restricted = aws.limit_to_region("us-east-1");

// TRY TO DELETE PRODUCTION DATABASE (should fail)
delete_db("prod-db-us-west-2", using aws)?;
// COMPILER ERROR: use of moved value 'aws'
```

**The Value:**
> "When you slice a capability, the original is consumed. You can't accidentally use the high-privilege version later. This is least privilege, enforced by the type system."

**Why v0.2:** Requires **Issue 010.5 (Capability Attenuation)** - new scope for capability type operations. Estimated 5-7 days, Phase 3.

---

## Demo 6: AI Incident Response Workflow

**Use Case:** Automated incident response with traceable AI decision-making

**Target Audience:** SRE teams, Security Operations, DevOps

**Why deferred to v0.2:**
- Complex scenario requires extensive faking/mocking
- Needs polished multi-step workflow presentation
- v0.1 focus is proving core capabilities
- Can build on Demo 2 (Meta-Agent) patterns

**What it would show:**
- AI analyzing service logs
- Generating remediation steps
- Auto-executing safe actions (with approval gates)
- Full audit trail of incident response

**Timeline:** Build after v0.1 proves market fit with simpler demos.

---

# Requirements Summary

## v0.1 Core Demos Requirements

**Language Features:**
- [x] Type checking with inference (Issues 001-005)
- [x] Functions with polymorphism (Issue 005)
- [x] Control flow (Issue 006)
- [x] ADTs with generics (Issue 007)
- [x] Effect system (Issue 008)
- [ ] Capability types (Issue 009) ← NEXT
- [ ] Affine types (Issue 010) ← CRITICAL for Demo 2
- [ ] WASM runtime (Issue 011)
- [ ] Effect tracing (Issue 011)
- [ ] Replay runner (Issue 011)

**Standard Library (Minimal):**
- `Result<T, E>`, `Option<T>`
- File I/O: `read_file`, `write_file`, `scan_directory`
- HTTP: `http_get`, `http_post`
- Git: `git_commit`
- GitHub: `github_create_pr`
- AI: `ai_generate`, `ai_generate_code`, `ai_review_code`
- Time: `log()`, `now()`
- Utilities: `hash()`, `len()`, `parse_json()`

**Tooling:**
- CLI: `strata run <file> --trace <output>`
- CLI: `strata replay <trace>`
- Effect trace JSON format
- Clear error messages

## v0.2 Expansion Demos Requirements

**Additional Features:**
- Issue 010.5: Capability Attenuation (NEW - 5-7 days)
- Finer-grained effect types (Pii effect)
- Effect transformations (scrubbers)
- More sophisticated capability operations

---

# Timeline

**v0.1 Demo Readiness:**
- Demo 1 (Deployment + Replay): Month 10 (all features complete)
- Demo 2 (Meta-Agent + Affine): Month 10 (Issue 010 is critical)
- Demo 3 (Time-Travel): Month 10 (replay infrastructure)
- **All three polished:** Month 11-12 (v0.1 launch window)

**v0.2 Expansion:**
- Demo 4 (PII Pipeline): Month 18-20
- Demo 5 (Blast Radius): Month 18-20 (after Issue 010.5)
- Demo 6 (Incident Response): Month 20-22

---

# Success Metrics

**v0.1 Launch:**
- All 3 core demos working
- Can present each in <8 minutes
- Clear "wow moments" identified
- Can run live without failures

**Post-Launch:**
- User recreates Demo 1 for their deployment
- Someone asks "Can I use this for my AI agents?"
- Conference talk accepted based on demos
- First "I rewrote my script in Strata" blog post

**v0.2 Validation:**
- Demo 4-6 working
- Shows breadth (AI ops, data privacy, cloud security)
- Someone builds use case we didn't anticipate
