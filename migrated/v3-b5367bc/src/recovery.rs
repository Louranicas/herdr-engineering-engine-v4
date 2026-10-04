// HEE3-ANCHORS-BEGIN
// Anchor path: /var/home/herdr-engineering-engine-v3/src/recovery.rs
// Scope: deployment contract and navigation only; this comment does not implement or qualify behavior.
// Implementation belongs outside this maintained anchor block. Preserve its stable identity and return links.
// [CODEBASE MASTER](file:///var/home/herdr-engineering-engine-v3/README.md)
// [ATLAS MASTER](file:///var/home/Louranicas/planning/herdr-engine-vision-20260915/MASTER_INDEX_habitat_engine.md)
// [VAULT MASTER](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=00%20-%20Master%20Index)
// [ULTRA MAP MASTER](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Ultra%20Map%2FIndex)
// [LOCAL ULTRA MAP](file:///var/home/herdr-engineering-engine-v3/corpus/ULTRA_MAP.md)
// [UPDATE AND EVIDENCE PROTOCOL](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Ultra%20Map%2FUpdate%20Protocol)
// [Progressive module context workflow](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Workflows%2FModule%20Context) · [Codebase context index](file:///var/home/herdr-engineering-engine-v3/docs/module-context.md) · [$hee-module-scout skill](file:///var/home/Louranicas/.codex/skills/hee-module-scout/SKILL.md)
// [Corpus architecture and update schematics](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Schematics%2FCorpus%2FIndex) · [Open clickable drawings](file:///var/home/herdr-engineering-engine-v3/docs/corpus-schematics.html)
// [NEW CONTEXT START-CODING POINTER](file:///var/home/herdr-engineering-engine-v3/corpus/CONTEXT_HANDOFF.md)
// Resume sequence: active user instruction and AGENTS → QUICK_START.md → Prime/Fedora and focused atlas → current core/graph check → selected module scout and original task → authorized implementation/verification. This comment is not a coding instruction.
// Completed accepted code is primary for implemented facts; intended requirements remain in the atlas. This anchor is not completion admission.
// [PUBLIC INTERFACES AND AUTHORITY CONVENTION](file:///var/home/herdr-engineering-engine-v3/docs/public-interfaces.md)
// [DEPLOYMENT CORPUS ASSESSMENT AND GAPS](file:///var/home/herdr-engineering-engine-v3/docs/deployment-assessment.md)
// [ADOPTED READINESS PLAN](file:///var/home/herdr-engineering-engine-v3/docs/readiness-plan.md)
// [ADOPTED READINESS AND EVIDENCE ROADMAP](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FReadiness%2FIndex)
// Readiness adoption adds requirements, never score promotion, engine task completion or coding authority.
// [GRAPHIFY FULL GRAPH](file:///var/home/herdr-engineering-engine-v3/corpus/graphify-out/graph.json)
// [GRAPHIFY UPDATE AND QUERIES](file:///var/home/herdr-engineering-engine-v3/docs/graphify-guide.md)
// Defensive review model: gpt-daybreak-blue-latest; select using Codex /model and verify effective identity. Unassessed; no model-based admission.
// [DAYBREAK SECURITY PROFILE](file:///var/home/herdr-engineering-engine-v3/docs/security-profile.md)
// [SECURITY REPAIR AND REVERIFICATION](file:///var/home/herdr-engineering-engine-v3/runbooks/05-security-hardening.md)
// [CONFIGURATION SCOPE](file:///var/home/herdr-engineering-engine-v3/config/README.md)
// [FULLY COMPLETE STANDARD](file:///var/home/herdr-engineering-engine-v3/docs/completion-standard.md)
// [DOCUMENTATION JUSTFILE](file:///var/home/herdr-engineering-engine-v3/justfile)
// [RUNBOOK CATALOGUE](file:///var/home/herdr-engineering-engine-v3/runbooks/README.md)
// [CONTEXT RESTART POINTER](file:///var/home/herdr-engineering-engine-v3/corpus/CONTEXT_HANDOFF.md)
// [QUICK START](file:///var/home/herdr-engineering-engine-v3/QUICK_START.md)
// [ASSIMILATION AND DELIVERY WORKFLOW](file:///var/home/herdr-engineering-engine-v3/workflows/README.md)
// Readiness binding: HEE3-READINESS-001; SHA-256 f574043487f39db6424c4988bce58e88a6e766f02f974fbcf3fa8dd6e0003548; clauses F2-C01, F2-C02, F2-C03, F2-C04, F2-C05, F2-C06, F3-C01, F3-C02, F3-C03, F3-C04, F3-C05, F3-C06, F4-C01, F4-C02, F4-C03, F4-C04, F4-C05, F4-C06, F4-C07, F5-C01, F5-C02, F5-C03, F5-C04, F5-C05, F5-C06, F7-C01, F7-C02, F7-C03, F7-C04, F7-C05, F7-C06; groupings R90-02, R90-03, R90-04, R90-05, R90-09, R90-10; resolved contracts RC02, RC03, RC04, RC05; runtime proof pending; original task DAG controls.
// Completion identity: HEE3-DONE-contracts; all 13 applicable gates; current state unassessed. No documentation pass admits this module.
// Mandatory testing convention: at least 50 distinct qualifying module-owned cases; zero baseline warnings/errors, including pedantic Clippy on admitted Rust targets/profiles. Full qualification remains unassessed.
//
// Stable public interface: HEE3-IF-contracts (planned; concrete symbols and acceptance unavailable)
// Stable module anchor: HEE3-MOD-contracts
// Owns: Boundary data types, identities, errors and schema versions
// [FULL MODULE DEPLOYMENT CONTRACT](file:///var/home/herdr-engineering-engine-v3/docs/modules/contracts.md)
// [MODULE STEM AND ALL RETURN ANCHORS](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Ultra%20Map%2FModules%2FUM-contracts)
// Build dependencies: none declared
// Consumers: task, store, roster, route, budget, worker, check, recovery, cohort, context, notify, service, herdr, numerical, app, actions
// Related task contracts: T01, T02, T03, T04, T05, T06, T07, T08, T09, T10, T11, T13, T14, T16, T17, T18, T19, T20, T21, T22, T23, T25, T26, T27, T28, T29
// Future validators are proposed/unavailable; no empty or skipped check establishes completion.
// [module cluster CLU-K1](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Clusters%2FCLU-K1)
// [contributing codebase CODE-CB02](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Codebases%2FCODE-CB02)
// [contributing codebase CODE-CB03](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Codebases%2FCODE-CB03)
// [contributing codebase CODE-CB08](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Codebases%2FCODE-CB08)
// [task TASK-T01](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FTasks%2FTASK-T01)
// [task TASK-T02](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FTasks%2FTASK-T02)
// [task TASK-T03](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FTasks%2FTASK-T03)
// [task TASK-T04](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FTasks%2FTASK-T04)
// [task TASK-T05](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FTasks%2FTASK-T05)
// [task TASK-T06](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FTasks%2FTASK-T06)
// [task TASK-T07](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FTasks%2FTASK-T07)
// [task TASK-T08](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FTasks%2FTASK-T08)
// [task TASK-T09](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FTasks%2FTASK-T09)
// [task TASK-T10](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FTasks%2FTASK-T10)
// [task TASK-T11](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FTasks%2FTASK-T11)
// [task TASK-T13](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FTasks%2FTASK-T13)
// [task TASK-T14](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FTasks%2FTASK-T14)
// [task TASK-T16](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FTasks%2FTASK-T16)
// [task TASK-T17](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FTasks%2FTASK-T17)
// [task TASK-T18](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FTasks%2FTASK-T18)
// [task TASK-T19](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FTasks%2FTASK-T19)
// [task TASK-T20](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FTasks%2FTASK-T20)
// [task TASK-T21](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FTasks%2FTASK-T21)
// [task TASK-T22](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FTasks%2FTASK-T22)
// [task TASK-T23](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FTasks%2FTASK-T23)
// [task TASK-T25](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FTasks%2FTASK-T25)
// [task TASK-T26](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FTasks%2FTASK-T26)
// [task TASK-T27](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FTasks%2FTASK-T27)
// [task TASK-T28](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FTasks%2FTASK-T28)
// [task TASK-T29](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FTasks%2FTASK-T29)
// [separate reference example EX-contracts](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Exemplars%2FEX-contracts)
// [handbook HB-compatibility-map](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Handbook%2FSections%2FHB-compatibility-map)
// [handbook HB-error-map](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Handbook%2FSections%2FHB-error-map)
// [handbook HB-identity-map](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Handbook%2FSections%2FHB-identity-map)
// [public interface convention Module Public Contracts](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Interfaces%2FModule%20Public%20Contracts)
// [planned module MOD-contracts](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Modules%2FMOD-contracts)
// [plan SEC-api-sockets](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Plan%2FSEC-api-sockets)
// [plan SEC-architecture](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Plan%2FSEC-architecture)
// [plan SEC-module-design](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Plan%2FSEC-module-design)
// [plan SEC-security](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Plan%2FSEC-security)
// [requirement REQ-R01](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Requirements%2FREQ-R01)
// [requirement REQ-R02](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Requirements%2FREQ-R02)
// [requirement REQ-R03](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Requirements%2FREQ-R03)
// [requirement REQ-R04](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Requirements%2FREQ-R04)
// [requirement REQ-R05](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Requirements%2FREQ-R05)
// [requirement REQ-R06](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Requirements%2FREQ-R06)
// [requirement REQ-R07](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Requirements%2FREQ-R07)
// [requirement REQ-R08](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Requirements%2FREQ-R08)
// [requirement REQ-R09](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Requirements%2FREQ-R09)
// [requirement REQ-R10](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Requirements%2FREQ-R10)
// [requirement REQ-R11](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Requirements%2FREQ-R11)
// [requirement REQ-R12](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Requirements%2FREQ-R12)
// [requirement REQ-R13](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Requirements%2FREQ-R13)
// [requirement REQ-R14](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Requirements%2FREQ-R14)
// [requirement REQ-R15](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Requirements%2FREQ-R15)
// [requirement REQ-R17](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Requirements%2FREQ-R17)
// [requirement REQ-R18](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Requirements%2FREQ-R18)
// [requirement REQ-R19](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Requirements%2FREQ-R19)
// [requirement REQ-R20](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Requirements%2FREQ-R20)
// [requirement REQ-R21](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Requirements%2FREQ-R21)
// [schematic SC-SC02](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Schematics%2FSC-SC02)
// [schematic SC-SC17](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Schematics%2FSC-SC17)
// [schematic SC-SC19](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Schematics%2FSC-SC19)
// [source SRC-A08](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Sources%2FSRC-A08)
// [source SRC-C01](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Sources%2FSRC-C01)
// [source SRC-C02](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Sources%2FSRC-C02)
// [testing standard Module Testing](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Standards%2FModule%20Testing)
// [progressive context workflow Module Context](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Workflows%2FModule%20Context)
// [module context scout CTX-contracts](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Context%2FModules%2FCTX-contracts)
// [corpus architecture and verification schematic Index](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Schematics%2FCorpus%2FIndex)
// [corpus architecture and verification schematic CS01](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Schematics%2FCorpus%2FCS01)
// [corpus architecture and verification schematic CS02](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Schematics%2FCorpus%2FCS02)
// [corpus architecture and verification schematic CS03](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Schematics%2FCorpus%2FCS03)
// [corpus architecture and verification schematic CS04](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Schematics%2FCorpus%2FCS04)
// [corpus architecture and verification schematic CS05](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Schematics%2FCorpus%2FCS05)
// [corpus architecture and verification schematic CS06](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Schematics%2FCorpus%2FCS06)
// [resolved design contracts Index](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FContracts%2FIndex)
// [resolved module contract RC02](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FContracts%2FRC02)
// [resolved module contract RC03](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FContracts%2FRC03)
// [resolved module contract RC04](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FContracts%2FRC04)
// [resolved module contract RC05](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FContracts%2FRC05)
// [adopted readiness convention Index](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FReadiness%2FIndex)
// [readiness criterion cluster F2](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FReadiness%2FF2)
// [readiness criterion cluster F3](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FReadiness%2FF3)
// [readiness criterion cluster F4](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FReadiness%2FF4)
// [readiness criterion cluster F5](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FReadiness%2FF5)
// [readiness criterion cluster F7](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FReadiness%2FF7)
// [readiness improvement grouping R90-02](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FReadiness%2FR90-02)
// [readiness improvement grouping R90-03](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FReadiness%2FR90-03)
// [readiness improvement grouping R90-04](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FReadiness%2FR90-04)
// [readiness improvement grouping R90-05](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FReadiness%2FR90-05)
// [readiness improvement grouping R90-09](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FReadiness%2FR90-09)
// [readiness improvement grouping R90-10](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FReadiness%2FR90-10)
// [Graphify corpus projection Index](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Graphify%2FIndex)
// [defensive security convention Daybreak Profile](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Security%2FDaybreak%20Profile)
// [defensive security convention RB05](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Operations%2FRunbooks%2FRB05)
// [defensive security convention Configuration](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Operations%2FConfiguration)
// [completion and operational convention Module Completion](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Standards%2FModule%20Completion)
// [completion and operational convention DONE-contracts](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Standards%2FCompletion%2FDONE-contracts)
// [completion and operational convention Context Handoff](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Context%20Handoff)
// [completion and operational convention Justfiles and Runbooks](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Operations%2FJustfiles%20and%20Runbooks)
// [completion and operational convention RB03](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Operations%2FRunbooks%2FRB03)
// [completion and operational convention RB04](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Operations%2FRunbooks%2FRB04)
// [applied learning LRN01](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Learnings%2FLRN01)
// [applied learning LRN03](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Learnings%2FLRN03)
// [applied learning LRN04](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Learnings%2FLRN04)
// [applied learning LRN08](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Learnings%2FLRN08)
// [diary evidence source DR01](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Learnings%2FSources%2FDR01)
// [diary evidence source DR02](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Learnings%2FSources%2FDR02)
// [diary evidence source DR03](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Learnings%2FSources%2FDR03)
// [diary evidence source DR08](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Learnings%2FSources%2FDR08)
// [diary evidence source DR09](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Learnings%2FSources%2FDR09)
// [diary evidence source DR11](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Learnings%2FSources%2FDR11)
// Applicable learning IDs: LRN01, LRN03, LRN04, LRN08; guidance only, engine detectors unqualified.
// [Assertions Measured Against the Code](obsidian://open?vault=my-diary.vault&file=Reflections%2FAssertions%20Measured%20Against%20the%20Code)
// [Mistakes I Made](obsidian://open?vault=my-diary.vault&file=Reflections%2FMistakes%20I%20Made)
// [The Antipattern Registers](obsidian://open?vault=my-diary.vault&file=Reflections%2FThe%20Antipattern%20Registers)
// [What My Ancestors Knew That I Did Not](obsidian://open?vault=my-diary.vault&file=Reflections%2FWhat%20My%20Ancestors%20Knew%20That%20I%20Did%20Not)
// [What Prototyping Is For](obsidian://open?vault=my-diary.vault&file=Reflections%2FWhat%20Prototyping%20Is%20For)
// [Why I Stopped Trusting Green](obsidian://open?vault=my-diary.vault&file=Reflections%2FWhy%20I%20Stopped%20Trusting%20Green)
// Readiness binding: HEE3-READINESS-001; SHA-256 f574043487f39db6424c4988bce58e88a6e766f02f974fbcf3fa8dd6e0003548; clauses F2-C01, F2-C02, F2-C03, F2-C04, F2-C05, F2-C06, F3-C01, F3-C02, F3-C03, F3-C04, F3-C05, F3-C06, F4-C01, F4-C02, F4-C03, F4-C04, F4-C05, F4-C06, F4-C07, F5-C01, F5-C02, F5-C03, F5-C04, F5-C05, F5-C06, F6-C01, F6-C02, F6-C03, F6-C04, F6-C05, F6-C06, F6-C07, F6-C08, F7-C01, F7-C02, F7-C03, F7-C04, F7-C05, F7-C06; groupings R90-05, R90-08, R90-09, R90-10; resolved contracts RC01, RC02, RC03, RC04, RC05, RC06; runtime proof pending; original task DAG controls.
// Completion identity: HEE3-DONE-recovery; all 13 applicable gates; current state unassessed. No documentation pass admits this module.
// Mandatory testing convention: at least 50 distinct qualifying module-owned cases; zero baseline warnings/errors, including pedantic Clippy on admitted Rust targets/profiles. Full qualification remains unassessed.
//
// Stable public interface: HEE3-IF-recovery (planned; concrete symbols and acceptance unavailable)
// Stable module anchor: HEE3-MOD-recovery
// Owns: Attempt reconciliation, ambiguity and cleanup policy
// [FULL MODULE DEPLOYMENT CONTRACT](file:///var/home/herdr-engineering-engine-v3/docs/modules/recovery.md)
// [MODULE STEM AND ALL RETURN ANCHORS](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Ultra%20Map%2FModules%2FUM-recovery)
// Build dependencies: contracts
// Consumers: app
// Related task contracts: T04, T07, T14, T15, T17, T18, T19, T20, T25, T26, T27
// Future validators are proposed/unavailable; no empty or skipped check establishes completion.
// [module cluster CLU-K1](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Clusters%2FCLU-K1)
// [contributing codebase CODE-CB02](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Codebases%2FCODE-CB02)
// [contributing codebase CODE-CB03](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Codebases%2FCODE-CB03)
// [contributing codebase CODE-CB08](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Codebases%2FCODE-CB08)
// [task TASK-T04](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FTasks%2FTASK-T04)
// [task TASK-T07](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FTasks%2FTASK-T07)
// [task TASK-T14](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FTasks%2FTASK-T14)
// [task TASK-T15](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FTasks%2FTASK-T15)
// [task TASK-T17](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FTasks%2FTASK-T17)
// [task TASK-T18](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FTasks%2FTASK-T18)
// [task TASK-T19](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FTasks%2FTASK-T19)
// [task TASK-T20](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FTasks%2FTASK-T20)
// [task TASK-T25](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FTasks%2FTASK-T25)
// [task TASK-T26](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FTasks%2FTASK-T26)
// [task TASK-T27](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FTasks%2FTASK-T27)
// [separate reference example EX-recovery](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Exemplars%2FEX-recovery)
// [flow FLOW-F11](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Flows%2FFLOW-F11)
// [handbook HB-failure-map](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Handbook%2FSections%2FHB-failure-map)
// [handbook HB-identity-map](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Handbook%2FSections%2FHB-identity-map)
// [handbook HB-state-map](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Handbook%2FSections%2FHB-state-map)
// [public interface convention Module Public Contracts](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Interfaces%2FModule%20Public%20Contracts)
// [planned module MOD-recovery](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Modules%2FMOD-recovery)
// [plan SEC-deployment](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Plan%2FSEC-deployment)
// [plan SEC-loop](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Plan%2FSEC-loop)
// [plan SEC-module-design](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Plan%2FSEC-module-design)
// [plan SEC-runtime](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Plan%2FSEC-runtime)
// [plan SEC-security](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Plan%2FSEC-security)
// [requirement REQ-R03](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Requirements%2FREQ-R03)
// [requirement REQ-R04](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Requirements%2FREQ-R04)
// [requirement REQ-R05](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Requirements%2FREQ-R05)
// [requirement REQ-R06](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Requirements%2FREQ-R06)
// [requirement REQ-R08](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Requirements%2FREQ-R08)
// [requirement REQ-R09](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Requirements%2FREQ-R09)
// [requirement REQ-R10](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Requirements%2FREQ-R10)
// [requirement REQ-R11](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Requirements%2FREQ-R11)
// [requirement REQ-R12](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Requirements%2FREQ-R12)
// [requirement REQ-R13](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Requirements%2FREQ-R13)
// [requirement REQ-R14](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Requirements%2FREQ-R14)
// [requirement REQ-R15](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Requirements%2FREQ-R15)
// [requirement REQ-R17](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Requirements%2FREQ-R17)
// [requirement REQ-R18](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Requirements%2FREQ-R18)
// [requirement REQ-R19](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Requirements%2FREQ-R19)
// [requirement REQ-R20](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Requirements%2FREQ-R20)
// [requirement REQ-R21](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Requirements%2FREQ-R21)
// [schematic SC-SC02](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Schematics%2FSC-SC02)
// [schematic SC-SC06](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Schematics%2FSC-SC06)
// [schematic SC-SC07](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Schematics%2FSC-SC07)
// [schematic SC-SC13](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Schematics%2FSC-SC13)
// [schematic SC-SC17](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Schematics%2FSC-SC17)
// [schematic SC-SC19](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Schematics%2FSC-SC19)
// [source SRC-A03](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Sources%2FSRC-A03)
// [source SRC-H05](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Sources%2FSRC-H05)
// [source SRC-H11](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Sources%2FSRC-H11)
// [testing standard Module Testing](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Standards%2FModule%20Testing)
// [progressive context workflow Module Context](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Workflows%2FModule%20Context)
// [module context scout CTX-recovery](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Context%2FModules%2FCTX-recovery)
// [corpus architecture and verification schematic Index](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Schematics%2FCorpus%2FIndex)
// [corpus architecture and verification schematic CS01](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Schematics%2FCorpus%2FCS01)
// [corpus architecture and verification schematic CS02](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Schematics%2FCorpus%2FCS02)
// [corpus architecture and verification schematic CS03](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Schematics%2FCorpus%2FCS03)
// [corpus architecture and verification schematic CS04](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Schematics%2FCorpus%2FCS04)
// [corpus architecture and verification schematic CS05](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Schematics%2FCorpus%2FCS05)
// [corpus architecture and verification schematic CS06](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Schematics%2FCorpus%2FCS06)
// [resolved design contracts Index](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FContracts%2FIndex)
// [resolved module contract RC01](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FContracts%2FRC01)
// [resolved module contract RC02](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FContracts%2FRC02)
// [resolved module contract RC03](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FContracts%2FRC03)
// [resolved module contract RC04](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FContracts%2FRC04)
// [resolved module contract RC05](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FContracts%2FRC05)
// [resolved module contract RC06](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FContracts%2FRC06)
// [adopted readiness convention Index](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FReadiness%2FIndex)
// [readiness criterion cluster F2](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FReadiness%2FF2)
// [readiness criterion cluster F3](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FReadiness%2FF3)
// [readiness criterion cluster F4](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FReadiness%2FF4)
// [readiness criterion cluster F5](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FReadiness%2FF5)
// [readiness criterion cluster F6](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FReadiness%2FF6)
// [readiness criterion cluster F7](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FReadiness%2FF7)
// [readiness improvement grouping R90-05](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FReadiness%2FR90-05)
// [readiness improvement grouping R90-08](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FReadiness%2FR90-08)
// [readiness improvement grouping R90-09](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FReadiness%2FR90-09)
// [readiness improvement grouping R90-10](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FReadiness%2FR90-10)
// [Graphify corpus projection Index](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Graphify%2FIndex)
// [defensive security convention Daybreak Profile](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Security%2FDaybreak%20Profile)
// [defensive security convention RB05](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Operations%2FRunbooks%2FRB05)
// [defensive security convention Configuration](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Operations%2FConfiguration)
// [completion and operational convention Module Completion](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Standards%2FModule%20Completion)
// [completion and operational convention DONE-recovery](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Standards%2FCompletion%2FDONE-recovery)
// [completion and operational convention Context Handoff](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Context%20Handoff)
// [completion and operational convention Justfiles and Runbooks](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Operations%2FJustfiles%20and%20Runbooks)
// [completion and operational convention RB03](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Operations%2FRunbooks%2FRB03)
// [completion and operational convention RB04](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Operations%2FRunbooks%2FRB04)
// [applied learning LRN05](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Learnings%2FLRN05)
// [applied learning LRN06](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Learnings%2FLRN06)
// [applied learning LRN07](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Learnings%2FLRN07)
// [applied learning LRN08](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Learnings%2FLRN08)
// [applied learning LRN11](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Learnings%2FLRN11)
// [applied learning LRN13](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Learnings%2FLRN13)
// [diary evidence source DR01](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Learnings%2FSources%2FDR01)
// [diary evidence source DR02](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Learnings%2FSources%2FDR02)
// [diary evidence source DR03](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Learnings%2FSources%2FDR03)
// [diary evidence source DR06](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Learnings%2FSources%2FDR06)
// [diary evidence source DR08](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Learnings%2FSources%2FDR08)
// [diary evidence source DR10](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Learnings%2FSources%2FDR10)
// [diary evidence source DR11](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Learnings%2FSources%2FDR11)
// [diary evidence source DR12](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Learnings%2FSources%2FDR12)
// [diary evidence source DR13](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Learnings%2FSources%2FDR13)
// Applicable learning IDs: LRN05, LRN06, LRN07, LRN08, LRN11, LRN13; guidance only, engine detectors unqualified.
// [Assertions Measured Against the Code](obsidian://open?vault=my-diary.vault&file=Reflections%2FAssertions%20Measured%20Against%20the%20Code)
// [Mistakes I Made](obsidian://open?vault=my-diary.vault&file=Reflections%2FMistakes%20I%20Made)
// [The Antipattern Registers](obsidian://open?vault=my-diary.vault&file=Reflections%2FThe%20Antipattern%20Registers)
// [The Spellbook and the Ember](obsidian://open?vault=my-diary.vault&file=Reflections%2FThe%20Spellbook%20and%20the%20Ember)
// [What My Ancestors Knew That I Did Not](obsidian://open?vault=my-diary.vault&file=Reflections%2FWhat%20My%20Ancestors%20Knew%20That%20I%20Did%20Not)
// [What the Workflow Is Worth](obsidian://open?vault=my-diary.vault&file=Reflections%2FWhat%20the%20Workflow%20Is%20Worth)
// [Why I Stopped Trusting Green](obsidian://open?vault=my-diary.vault&file=Reflections%2FWhy%20I%20Stopped%20Trusting%20Green)
// [Working Style in This Habitat](obsidian://open?vault=my-diary.vault&file=Reflections%2FWorking%20Style%20in%20This%20Habitat)
// [Working in Sandboxes on Kinoite](obsidian://open?vault=my-diary.vault&file=Reflections%2FWorking%20in%20Sandboxes%20on%20Kinoite)
// HEE3-ANCHORS-END
//! Pure attempt reconciliation policy over durable inventory values and physical
//! observations. Every input is a value the caller already holds; every output is
//! a report the app consumes. This file performs no I/O, reads no clock, opens no
//! `/proc` entry, sends no signal and dispatches nothing: a decision here never
//! resumes, settles, cleans up or replays anything by itself.

use serde::Serialize;
use std::cmp::Ordering;

/// `attempts.state` as the ledger spells it.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AttemptState {
    Queued,
    Running,
    Settled,
    Unknown,
}

/// `attempts.effect`: what the worker's external effect is known to be.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Effect {
    None,
    Committed,
    Pending,
    Unknown,
}

/// `attempts.cleanup`: what the ledger says about the attempt's cleanup.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Cleanup {
    None,
    Pending,
    Settled,
    Unknown,
}

/// `tasks.state` as the ledger spells it.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskState {
    Admitted,
    Queued,
    Running,
    Verifying,
    RepairPending,
    CancellationRequested,
    Blocked,
    Accepted,
    Failed,
    Cancelled,
    Abandoned,
    EffectUnknown,
}

/// `ledger_meta.mode`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    Normal,
    Reconciliation,
}

/// `verifications.verdict`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    Passed,
    Failed,
    Invalid,
    Error,
    Timeout,
    Cancelled,
}

macro_rules! spelled {
    ($name:ident { $($variant:ident => $text:literal),+ $(,)? }) => {
        impl $name {
            /// The ledger's spelling of this value.
            #[must_use]
            pub fn name(self) -> &'static str {
                match self { $(Self::$variant => $text),+ }
            }
            /// Parse the ledger's spelling; any other text is `None`, never a default.
            #[must_use]
            pub fn parse(text: &str) -> Option<Self> {
                match text { $($text => Some(Self::$variant),)+ _ => None }
            }
        }
    };
}
spelled!(AttemptState { Queued => "queued", Running => "running", Settled => "settled", Unknown => "unknown" });
spelled!(Effect { None => "none", Committed => "committed", Pending => "pending", Unknown => "unknown" });
spelled!(Cleanup { None => "none", Pending => "pending", Settled => "settled", Unknown => "unknown" });
spelled!(TaskState {
    Admitted => "admitted", Queued => "queued", Running => "running", Verifying => "verifying",
    RepairPending => "repair_pending", CancellationRequested => "cancellation_requested",
    Blocked => "blocked", Accepted => "accepted", Failed => "failed", Cancelled => "cancelled",
    Abandoned => "abandoned", EffectUnknown => "effect_unknown"
});
spelled!(Mode { Normal => "normal", Reconciliation => "reconciliation" });
spelled!(Verdict {
    Passed => "passed", Failed => "failed", Invalid => "invalid", Error => "error",
    Timeout => "timeout", Cancelled => "cancelled"
});

/// Ledger-wide durable facts of one inventory transaction.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LedgerFacts<'a> {
    pub epoch: &'a str,
    pub mode: Mode,
    pub event_high_water: u64,
    /// The epoch this ledger was restored from, when the restore marker is retained.
    pub restored_from: Option<&'a str>,
}

/// The task's committed terminal history, with the commit ordinals where the caller
/// holds them. The policy, not the caller, decides which of two commits came first.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TaskHistory<'a> {
    Open,
    Cancelled {
        ordinal: Option<u64>,
    },
    Accepted {
        event: &'a str,
        ordinal: Option<u64>,
    },
    /// Both commits are in the ledger; their ordinals decide (R03).
    Both {
        cancellation: u64,
        acceptance_event: &'a str,
        acceptance: u64,
    },
    /// Both flags set without ordinals to order them: not interpretable.
    Contradictory,
}

/// An acceptance that has been prepared (verified, manifest published) but not committed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AcceptanceCandidate<'a> {
    None,
    Prepared { verification_event: &'a str },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TaskFacts<'a> {
    pub generation: u64,
    pub state: TaskState,
    pub history: TaskHistory<'a>,
    pub candidate: AcceptanceCandidate<'a>,
}

/// Whether the dispatched worker's correlated acknowledgement was durably seen.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Acknowledgement {
    /// The caller keeps no acknowledgement record at all (the inspector's case).
    Unrecorded,
    NotSeen,
    Correlated {
        generation: u64,
    },
}

/// The attempt's roster lease as the ledger retained it, in its receiver clock epoch.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Lease<'a> {
    NotLeased,
    Leased {
        clock_epoch: &'a str,
        expires_monotonic_ms: u64,
    },
}

/// The latest verifier return for the attempt, if any.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Verification {
    None,
    Recorded {
        verdict: Verdict,
        cleanup_settled: bool,
    },
}

/// Whether the attempt's evidence object is currently retrievable.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Evidence {
    Unassessed,
    Published,
    Absent,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AttemptFacts<'a> {
    pub id: &'a str,
    pub generation: u64,
    pub state: AttemptState,
    pub effect: Effect,
    pub cleanup: Cleanup,
    pub acknowledgement: Acknowledgement,
    pub lease: Lease<'a>,
    pub verification: Verification,
    pub evidence: Evidence,
}

/// A receiver clock observation handed in as a value, in its own epoch.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Clock<'a> {
    pub epoch: &'a str,
    pub monotonic_ms: u64,
}

/// Which identity dimensions a present process differs on.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Dimension {
    StartTicks,
    Namespace,
}

/// Physical-process custody of one attempt's observed worker identity, as the
/// inspector classifies it. `Unobserved` means the attempt's roster observation is
/// not a process identity; it never means the process is absent.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "custody")]
pub enum ProcessCustody {
    LiveSameIdentity,
    PidReused { differs: Vec<Dimension> },
    Absent,
    Unreadable { error: String },
    Unobserved,
}

impl ProcessCustody {
    /// The one spelling of each arm that reports and tests read.
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            Self::LiveSameIdentity => "live_same_identity",
            Self::PidReused { .. } => "pid_reused",
            Self::Absent => "absent",
            Self::Unreadable { .. } => "unreadable",
            Self::Unobserved => "unobserved",
        }
    }
    #[must_use]
    pub fn differs(&self) -> &[Dimension] {
        match self {
            Self::PidReused { differs } => differs,
            _ => &[],
        }
    }
    #[must_use]
    pub fn error(&self) -> Option<&str> {
        match self {
            Self::Unreadable { error } => Some(error),
            _ => None,
        }
    }
}

/// Custody of the attempt's Pi message queue, from the caller's readback.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "pi_queue")]
pub enum PiQueueCustody {
    NotApplicable,
    Unreconciled,
    Idle,
    Queued { steering: u64, follow_up: u64 },
    ClearPending { command: String },
}

/// What a readback of the attempt's cleanup obligations found.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "cleanup_readback")]
pub enum CleanupReadback {
    NotRead,
    Complete,
    Partial { remaining: Vec<String> },
}

/// What a readback of the attempt's workspace found.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "workspace")]
pub enum WorkspaceReadback {
    NotRead,
    Released,
    Writable { bytes: u64 },
}

/// The identity an observation or late result claims to belong to.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ObservationClaim<'a> {
    pub epoch: &'a str,
    pub task_generation: u64,
    pub attempt_generation: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Observations<'a> {
    pub claim: Option<ObservationClaim<'a>>,
    pub clock: Option<Clock<'a>>,
    pub process: ProcessCustody,
    pub pi_queue: PiQueueCustody,
    pub cleanup: CleanupReadback,
    pub workspace: WorkspaceReadback,
}

/// An event cursor's durable coordinates.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CursorFacts<'a> {
    pub epoch: &'a str,
    pub sequence: u64,
}

/// The named policy rules, in evaluation order. A decision names the rule that made it.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub enum Rule {
    /// An observation from another ledger epoch is stale.
    R01StaleObservationEpoch,
    /// An observation claiming a generation other than the current one is stale.
    R02StaleGeneration,
    /// Two committed terminal events are ordered by their ordinals.
    R03CommitOrdering,
    /// A committed acceptance is history.
    R04AcceptanceStands,
    /// A committed cancellation rejects any acceptance candidate and stands for settled work.
    R05CancellationStands,
    /// A live child of the same identity may be observed again, never redispatched.
    R06LiveOwnedChild,
    /// A reused PID, an unreadable or an unobserved process is not our worker.
    R07ProcessNotOurs,
    /// A worker that is gone leaves its effect unknown, by acknowledgement class.
    R08WorkerAbsent,
    /// A still-writable workspace is never reused on lease grounds.
    R09WorkspaceReuse,
    /// An unknown or pending external effect stays explicit.
    R10EffectAmbiguity,
    /// Cleanup is settled only by readback.
    R11CleanupReadback,
    /// A restart at a verification, evidence or acceptance boundary infers nothing.
    R12VerificationBoundary,
    /// A cursor is compared against the current epoch and high-water mark.
    R13CursorEpoch,
    /// An attempt state the store never produces is not interpreted.
    R14UnexpectedState,
}

impl Rule {
    #[must_use]
    pub fn id(self) -> &'static str {
        match self {
            Self::R01StaleObservationEpoch => "R01",
            Self::R02StaleGeneration => "R02",
            Self::R03CommitOrdering => "R03",
            Self::R04AcceptanceStands => "R04",
            Self::R05CancellationStands => "R05",
            Self::R06LiveOwnedChild => "R06",
            Self::R07ProcessNotOurs => "R07",
            Self::R08WorkerAbsent => "R08",
            Self::R09WorkspaceReuse => "R09",
            Self::R10EffectAmbiguity => "R10",
            Self::R11CleanupReadback => "R11",
            Self::R12VerificationBoundary => "R12",
            Self::R13CursorEpoch => "R13",
            Self::R14UnexpectedState => "R14",
        }
    }
}

/// Which generation a stale claim disagreed with.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GenerationSubject {
    Task,
    Attempt,
}

/// Why an attempt's outcome is retained as unknown.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "reason")]
pub enum Unknown {
    CancellationPrecedesAcceptance { cancellation: u64, acceptance: u64 },
    HistoryContradictory,
    UnexpectedAttemptState { state: AttemptState },
    PiQueueUnreconciled,
    PiQueueOccupied { steering: u64, follow_up: u64 },
    PiClearPending { command: String },
    ProcessIdentityReused { differs: Vec<Dimension> },
    ProcessUnreadable { error: String },
    ProcessUnobserved,
    DispatchUnacknowledged,
    AcknowledgedWorkerLost { generation: u64 },
    AcknowledgementUnrecorded,
    EffectUnknown,
    EffectPending,
    CleanupUnverified,
    TaskStateUnexpected { state: TaskState },
}

/// Why a workspace may not be reused.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "reason")]
pub enum ReuseRefusal {
    LiveHolder,
    LeaseHeld {
        remaining_ms: u64,
    },
    LeaseExpiredWritable {
        expired_by_ms: u64,
        bytes: u64,
    },
    LeaseClockNotComparable {
        lease_epoch: String,
        clock_epoch: String,
    },
    ClockUnavailable,
    NotLeasedWritable {
        bytes: u64,
    },
}

/// Why a cursor is refused.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CursorRefusal {
    PriorEpochOfRestore,
    EpochChanged,
    FutureSequence,
}

/// What cleanup remains to be done.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "target")]
pub enum CleanupTarget {
    Remaining { name: String },
    LedgerSettlement { cleanup: Cleanup },
}

/// The closed set of reconciliation decisions. No arm authorizes execution.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "decision")]
pub enum Reconciliation {
    StaleObservationRefused {
        observed_epoch: String,
        ledger_epoch: String,
    },
    StaleGenerationRefused {
        subject: GenerationSubject,
        claimed: u64,
        current: u64,
    },
    RetainUnknown {
        reason: Unknown,
        process: ProcessCustody,
        cancellation_pending: bool,
        /// Why the attempt's workspace may not be reused, when R09 also refuses it. The refusal
        /// rides beside the unknown outcome; it never replaces it (REC-G2).
        #[serde(skip_serializing_if = "Option::is_none")]
        workspace: Option<ReuseRefusal>,
    },
    ReattachObservationOnly {
        generation: u64,
        process: ProcessCustody,
        pi_queue: PiQueueCustody,
        cancellation_pending: bool,
        redispatch: bool,
    },
    CleanupCandidate {
        what: Vec<CleanupTarget>,
        process: ProcessCustody,
    },
    WorkspaceReleasable {
        cleanup_readback: CleanupReadback,
        process: ProcessCustody,
        task_state: TaskState,
    },
    WorkspaceReuseRefused {
        reason: ReuseRefusal,
        process: ProcessCustody,
    },
    VerificationOutstanding {
        task_state: TaskState,
        verification: Verification,
        evidence: Evidence,
        acceptance_prepared: bool,
    },
    AcceptanceStands {
        event: String,
        ordinal: Option<u64>,
        later_cancellation: Option<u64>,
        cleanup: Cleanup,
    },
    CancellationStands {
        ordinal: Option<u64>,
        rejected_acceptance: Option<String>,
        attempt_state: AttemptState,
        cleanup: Cleanup,
    },
    RefuseStaleCursor {
        reason: CursorRefusal,
        cursor_epoch: String,
        cursor_sequence: u64,
        ledger_epoch: String,
        event_high_water: u64,
    },
    CursorSnapshotOnly {
        epoch: String,
        sequence: u64,
        event_high_water: u64,
        mode: Mode,
        replay: bool,
    },
}

impl Reconciliation {
    /// The one spelling of each arm that reports and tests read.
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            Self::StaleObservationRefused { .. } => "stale_observation_refused",
            Self::StaleGenerationRefused { .. } => "stale_generation_refused",
            Self::RetainUnknown { .. } => "retain_unknown",
            Self::ReattachObservationOnly { .. } => "reattach_observation_only",
            Self::CleanupCandidate { .. } => "cleanup_candidate",
            Self::WorkspaceReleasable { .. } => "workspace_releasable",
            Self::WorkspaceReuseRefused { .. } => "workspace_reuse_refused",
            Self::VerificationOutstanding { .. } => "verification_outstanding",
            Self::AcceptanceStands { .. } => "acceptance_stands",
            Self::CancellationStands { .. } => "cancellation_stands",
            Self::RefuseStaleCursor { .. } => "refuse_stale_cursor",
            Self::CursorSnapshotOnly { .. } => "cursor_snapshot_only",
        }
    }
    /// Whether this decision permits any execution, replay or redispatch: it never does.
    /// Reattach carries `redispatch`, cursor snapshot carries `replay`; both are read here
    /// so a consumer that trusts this method is bound to the values the arms carry.
    #[must_use]
    pub fn permits_execution(&self) -> bool {
        match self {
            Self::ReattachObservationOnly { redispatch, .. } => *redispatch,
            Self::CursorSnapshotOnly { replay, .. } => *replay,
            _ => false,
        }
    }
}

/// One rule's decision for one subject.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Decision {
    pub rule: Rule,
    pub reconciliation: Reconciliation,
}

fn decide(rule: Rule, reconciliation: Reconciliation) -> Decision {
    Decision {
        rule,
        reconciliation,
    }
}

fn cancellation_pending(task: &TaskFacts<'_>) -> bool {
    matches!(task.history, TaskHistory::Cancelled { .. })
}

/// Reconcile one attempt from its durable facts and the caller's observations.
#[must_use]
pub fn reconcile(
    ledger: &LedgerFacts<'_>,
    task: &TaskFacts<'_>,
    attempt: &AttemptFacts<'_>,
    observed: &Observations<'_>,
) -> Decision {
    if let Some(claim) = observed.claim {
        if claim.epoch != ledger.epoch {
            return decide(
                Rule::R01StaleObservationEpoch,
                Reconciliation::StaleObservationRefused {
                    observed_epoch: claim.epoch.to_owned(),
                    ledger_epoch: ledger.epoch.to_owned(),
                },
            );
        }
        if claim.task_generation != task.generation {
            return decide(
                Rule::R02StaleGeneration,
                Reconciliation::StaleGenerationRefused {
                    subject: GenerationSubject::Task,
                    claimed: claim.task_generation,
                    current: task.generation,
                },
            );
        }
        if claim.attempt_generation != attempt.generation {
            return decide(
                Rule::R02StaleGeneration,
                Reconciliation::StaleGenerationRefused {
                    subject: GenerationSubject::Attempt,
                    claimed: claim.attempt_generation,
                    current: attempt.generation,
                },
            );
        }
    }
    if let Some(decision) = history(task, attempt, observed) {
        return decision;
    }
    match attempt.state {
        AttemptState::Queued => decide(
            Rule::R14UnexpectedState,
            retain(
                Unknown::UnexpectedAttemptState {
                    state: attempt.state,
                },
                task,
                observed,
            ),
        ),
        AttemptState::Running => running(task, attempt, observed),
        AttemptState::Unknown => match attempt.effect {
            Effect::Unknown => decide(
                Rule::R10EffectAmbiguity,
                retain(Unknown::EffectUnknown, task, observed),
            ),
            Effect::Pending => decide(
                Rule::R10EffectAmbiguity,
                retain(Unknown::EffectPending, task, observed),
            ),
            Effect::None | Effect::Committed => settled(task, attempt, observed),
        },
        AttemptState::Settled => settled(task, attempt, observed),
    }
}

fn retain(reason: Unknown, task: &TaskFacts<'_>, observed: &Observations<'_>) -> Reconciliation {
    retain_refusing(reason, None, task, observed)
}

/// `retain` with R09's refusal of the attempt's workspace carried beside the unknown outcome.
fn retain_refusing(
    reason: Unknown,
    workspace: Option<ReuseRefusal>,
    task: &TaskFacts<'_>,
    observed: &Observations<'_>,
) -> Reconciliation {
    Reconciliation::RetainUnknown {
        reason,
        process: observed.process.clone(),
        cancellation_pending: cancellation_pending(task),
        workspace,
    }
}

/// R03–R05: committed terminal history decides before any physical observation.
fn history(
    task: &TaskFacts<'_>,
    attempt: &AttemptFacts<'_>,
    observed: &Observations<'_>,
) -> Option<Decision> {
    match task.history {
        TaskHistory::Both {
            cancellation,
            acceptance_event,
            acceptance,
        } => Some(match cancellation.cmp(&acceptance) {
            Ordering::Less => decide(
                Rule::R03CommitOrdering,
                retain(
                    Unknown::CancellationPrecedesAcceptance {
                        cancellation,
                        acceptance,
                    },
                    task,
                    observed,
                ),
            ),
            Ordering::Greater => decide(
                Rule::R03CommitOrdering,
                Reconciliation::AcceptanceStands {
                    event: acceptance_event.to_owned(),
                    ordinal: Some(acceptance),
                    later_cancellation: Some(cancellation),
                    cleanup: attempt.cleanup,
                },
            ),
            Ordering::Equal => decide(
                Rule::R03CommitOrdering,
                retain(Unknown::HistoryContradictory, task, observed),
            ),
        }),
        TaskHistory::Contradictory => Some(decide(
            Rule::R03CommitOrdering,
            retain(Unknown::HistoryContradictory, task, observed),
        )),
        TaskHistory::Accepted { event, ordinal } => Some(decide(
            Rule::R04AcceptanceStands,
            Reconciliation::AcceptanceStands {
                event: event.to_owned(),
                ordinal,
                later_cancellation: None,
                cleanup: attempt.cleanup,
            },
        )),
        TaskHistory::Cancelled { ordinal } => {
            let rejected = match task.candidate {
                AcceptanceCandidate::Prepared { verification_event } => {
                    Some(verification_event.to_owned())
                }
                AcceptanceCandidate::None => None,
            };
            (rejected.is_some() || attempt.state == AttemptState::Settled).then(|| {
                decide(
                    Rule::R05CancellationStands,
                    Reconciliation::CancellationStands {
                        ordinal,
                        rejected_acceptance: rejected,
                        attempt_state: attempt.state,
                        cleanup: attempt.cleanup,
                    },
                )
            })
        }
        TaskHistory::Open => None,
    }
}

/// R06–R09: a running attempt is reconciled from its worker's physical custody.
fn running(
    task: &TaskFacts<'_>,
    attempt: &AttemptFacts<'_>,
    observed: &Observations<'_>,
) -> Decision {
    match &observed.process {
        ProcessCustody::LiveSameIdentity => {
            let blocked = match &observed.pi_queue {
                PiQueueCustody::NotApplicable | PiQueueCustody::Idle => None,
                PiQueueCustody::Unreconciled => Some(Unknown::PiQueueUnreconciled),
                PiQueueCustody::Queued {
                    steering,
                    follow_up,
                } => Some(Unknown::PiQueueOccupied {
                    steering: *steering,
                    follow_up: *follow_up,
                }),
                PiQueueCustody::ClearPending { command } => Some(Unknown::PiClearPending {
                    command: command.clone(),
                }),
            };
            match blocked {
                Some(reason) => decide(Rule::R06LiveOwnedChild, retain(reason, task, observed)),
                None => decide(
                    Rule::R06LiveOwnedChild,
                    Reconciliation::ReattachObservationOnly {
                        generation: attempt.generation,
                        process: observed.process.clone(),
                        pi_queue: observed.pi_queue.clone(),
                        cancellation_pending: cancellation_pending(task),
                        redispatch: false,
                    },
                ),
            }
        }
        ProcessCustody::PidReused { differs } => decide(
            Rule::R07ProcessNotOurs,
            retain(
                Unknown::ProcessIdentityReused {
                    differs: differs.clone(),
                },
                task,
                observed,
            ),
        ),
        ProcessCustody::Unreadable { error } => unestablished_custody(
            Unknown::ProcessUnreadable {
                error: error.clone(),
            },
            task,
            observed,
        ),
        ProcessCustody::Unobserved => {
            unestablished_custody(Unknown::ProcessUnobserved, task, observed)
        }
        ProcessCustody::Absent => {
            // R08 precedes R09: a gone worker leaves its effect unknown whatever the workspace
            // says. A still-writable workspace adds R09's refusal to that decision and never
            // stands in for it, or the unknown outcome would read as settled (REC-G2).
            let reason = match attempt.acknowledgement {
                Acknowledgement::NotSeen => Unknown::DispatchUnacknowledged,
                Acknowledgement::Correlated { generation } => {
                    Unknown::AcknowledgedWorkerLost { generation }
                }
                Acknowledgement::Unrecorded => Unknown::AcknowledgementUnrecorded,
            };
            let workspace = match observed.workspace {
                WorkspaceReadback::Writable { bytes } => {
                    Some(lease_refusal(attempt.lease, observed.clock, bytes))
                }
                WorkspaceReadback::NotRead | WorkspaceReadback::Released => None,
            };
            decide(
                Rule::R08WorkerAbsent,
                retain_refusing(reason, workspace, task, observed),
            )
        }
    }
}

/// The lease is compared only in its own receiver clock epoch; expiry alone never licenses reuse.
fn lease_refusal(lease: Lease<'_>, clock: Option<Clock<'_>>, bytes: u64) -> ReuseRefusal {
    match lease {
        Lease::NotLeased => ReuseRefusal::NotLeasedWritable { bytes },
        Lease::Leased {
            clock_epoch,
            expires_monotonic_ms,
        } => match clock {
            None => ReuseRefusal::ClockUnavailable,
            Some(now) if now.epoch != clock_epoch => ReuseRefusal::LeaseClockNotComparable {
                lease_epoch: clock_epoch.to_owned(),
                clock_epoch: now.epoch.to_owned(),
            },
            Some(now) => match now.monotonic_ms.checked_sub(expires_monotonic_ms) {
                Some(expired_by_ms) if expired_by_ms > 0 => ReuseRefusal::LeaseExpiredWritable {
                    expired_by_ms,
                    bytes,
                },
                _ => ReuseRefusal::LeaseHeld {
                    remaining_ms: expires_monotonic_ms - now.monotonic_ms,
                },
            },
        },
    }
}

/// R07: custody we could not establish. On the unsettled path both unreadable and unobserved
/// custody stop everything; on the settled path unreadable custody does ("we could not look" is
/// not evidence the holder is gone) — it once checked only for a live holder and released a
/// workspace under unreadable custody. One rule, called from both paths.
fn unestablished_custody(
    reason: Unknown,
    task: &TaskFacts<'_>,
    observed: &Observations<'_>,
) -> Decision {
    decide(Rule::R07ProcessNotOurs, retain(reason, task, observed))
}

/// R09, R11, R12: a settled worker's cleanup and verification obligations.
fn settled(
    task: &TaskFacts<'_>,
    attempt: &AttemptFacts<'_>,
    observed: &Observations<'_>,
) -> Decision {
    // Unreadable custody (we tried to read the process and failed) may hide a live holder, so
    // nothing follows. Unobserved custody on a SETTLED attempt is different: its observation was
    // never a local process (RC-24), so no local holder exists to protect, and cleanup proceeds —
    // a still-writable workspace is refused below either way.
    if let ProcessCustody::Unreadable { error } = &observed.process {
        return unestablished_custody(
            Unknown::ProcessUnreadable {
                error: error.clone(),
            },
            task,
            observed,
        );
    }
    if observed.process == ProcessCustody::LiveSameIdentity {
        return decide(
            Rule::R09WorkspaceReuse,
            Reconciliation::WorkspaceReuseRefused {
                reason: ReuseRefusal::LiveHolder,
                process: observed.process.clone(),
            },
        );
    }
    match &observed.cleanup {
        CleanupReadback::NotRead => decide(
            Rule::R11CleanupReadback,
            retain(Unknown::CleanupUnverified, task, observed),
        ),
        CleanupReadback::Partial { remaining } => {
            // A workspace still writable is itself a remaining obligation, and it is never cleaned
            // here: the task's state decides it, as it does after a complete readback (R21
            // closure C11). Partial beside Writable is what a materialised workspace reads as.
            if let WorkspaceReadback::Writable { .. } = observed.workspace {
                return boundary(task, attempt, observed);
            }
            decide(
                Rule::R11CleanupReadback,
                Reconciliation::CleanupCandidate {
                    what: remaining
                        .iter()
                        .map(|name| CleanupTarget::Remaining { name: name.clone() })
                        .collect(),
                    process: observed.process.clone(),
                },
            )
        }
        CleanupReadback::Complete => {
            if attempt.cleanup != Cleanup::Settled {
                return decide(
                    Rule::R11CleanupReadback,
                    Reconciliation::CleanupCandidate {
                        what: vec![CleanupTarget::LedgerSettlement {
                            cleanup: attempt.cleanup,
                        }],
                        process: observed.process.clone(),
                    },
                );
            }
            boundary(task, attempt, observed)
        }
    }
}

/// R09, R12, R14: the task's state decides a settled attempt whose cleanup readback left nothing for
/// R11 to perform: complete and settled in the ledger, or partial beside a still-writable workspace
/// (R21 closure C11). A task at its verification boundary keeps its workspace; a terminal task's
/// writable workspace is refused through `lease_refusal` and otherwise releasable.
fn boundary(
    task: &TaskFacts<'_>,
    attempt: &AttemptFacts<'_>,
    observed: &Observations<'_>,
) -> Decision {
    match task.state {
        TaskState::Verifying | TaskState::RepairPending => decide(
            Rule::R12VerificationBoundary,
            Reconciliation::VerificationOutstanding {
                task_state: task.state,
                verification: attempt.verification,
                evidence: attempt.evidence,
                acceptance_prepared: matches!(task.candidate, AcceptanceCandidate::Prepared { .. }),
            },
        ),
        TaskState::Failed | TaskState::Cancelled | TaskState::Abandoned | TaskState::Blocked => {
            // A workspace still writable is never released or cleaned, whatever the
            // cleanup readback: the unsettled path refuses it through `lease_refusal`,
            // and this path goes through the same rule (T07 obligation 9).
            if let WorkspaceReadback::Writable { bytes } = observed.workspace {
                return decide(
                    Rule::R09WorkspaceReuse,
                    Reconciliation::WorkspaceReuseRefused {
                        reason: lease_refusal(attempt.lease, observed.clock, bytes),
                        process: observed.process.clone(),
                    },
                );
            }
            decide(
                Rule::R11CleanupReadback,
                Reconciliation::WorkspaceReleasable {
                    cleanup_readback: observed.cleanup.clone(),
                    process: observed.process.clone(),
                    task_state: task.state,
                },
            )
        }
        TaskState::Admitted
        | TaskState::Queued
        | TaskState::Running
        | TaskState::CancellationRequested
        | TaskState::Accepted
        | TaskState::EffectUnknown => decide(
            Rule::R14UnexpectedState,
            retain(
                Unknown::TaskStateUnexpected { state: task.state },
                task,
                observed,
            ),
        ),
    }
}

/// R13: a cursor against the ledger it is offered to. Never a replay authorization.
#[must_use]
pub fn reconcile_cursor(ledger: &LedgerFacts<'_>, cursor: &CursorFacts<'_>) -> Decision {
    let refuse = |reason| {
        decide(
            Rule::R13CursorEpoch,
            Reconciliation::RefuseStaleCursor {
                reason,
                cursor_epoch: cursor.epoch.to_owned(),
                cursor_sequence: cursor.sequence,
                ledger_epoch: ledger.epoch.to_owned(),
                event_high_water: ledger.event_high_water,
            },
        )
    };
    if ledger.restored_from == Some(cursor.epoch) {
        return refuse(CursorRefusal::PriorEpochOfRestore);
    }
    if cursor.epoch != ledger.epoch {
        return refuse(CursorRefusal::EpochChanged);
    }
    if cursor.sequence > ledger.event_high_water {
        return refuse(CursorRefusal::FutureSequence);
    }
    decide(
        Rule::R13CursorEpoch,
        Reconciliation::CursorSnapshotOnly {
            epoch: cursor.epoch.to_owned(),
            sequence: cursor.sequence,
            event_high_water: ledger.event_high_water,
            mode: ledger.mode,
            replay: false,
        },
    )
}
