// HEE3-ANCHORS-BEGIN
// Anchor path: /var/home/herdr-engineering-engine-v3/src/numerical.rs
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
// Readiness binding: HEE3-READINESS-001; SHA-256 f574043487f39db6424c4988bce58e88a6e766f02f974fbcf3fa8dd6e0003548; clauses F1-C01, F1-C02, F1-C03, F1-C04, F1-C05, F2-C01, F2-C02, F2-C03, F2-C04, F2-C05, F2-C06, F3-C01, F3-C02, F3-C03, F3-C04, F3-C05, F3-C06, F4-C01, F4-C02, F4-C03, F4-C04, F4-C05, F4-C06, F4-C07, F5-C01, F5-C02, F5-C03, F5-C04, F5-C05, F5-C06, F7-C01, F7-C02, F7-C03, F7-C04, F7-C05, F7-C06; groupings R90-01, R90-03, R90-07, R90-09, R90-10; resolved contracts RC01, RC02, RC03, RC04, RC05; runtime proof pending; original task DAG controls.
// Completion identity: HEE3-DONE-numerical; all 13 applicable gates; current state unassessed. No documentation pass admits this module.
// Mandatory testing convention: at least 50 distinct qualifying module-owned cases; zero baseline warnings/errors, including pedantic Clippy on admitted Rust targets/profiles. Full qualification remains unassessed.
//
// Stable public interface: HEE3-IF-numerical (planned; concrete symbols and acceptance unavailable)
// Stable module anchor: HEE3-MOD-numerical
// Owns: Bounded Rust–Julia process and data boundary
// [FULL MODULE DEPLOYMENT CONTRACT](file:///var/home/herdr-engineering-engine-v3/docs/modules/numerical.md)
// [MODULE STEM AND ALL RETURN ANCHORS](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Ultra%20Map%2FModules%2FUM-numerical)
// Build dependencies: contracts
// Consumers: app
// Related task contracts: T12, T14, T15, T17, T18, T19, T20, T21, T22, T23, T24, T25, T26, T27
// Future validators are proposed/unavailable; no empty or skipped check establishes completion.
// [module cluster CLU-K4](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Clusters%2FCLU-K4)
// [contributing codebase CODE-CB01](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Codebases%2FCODE-CB01)
// [contributing codebase CODE-CB02](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Codebases%2FCODE-CB02)
// [contributing codebase CODE-CB05](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Codebases%2FCODE-CB05)
// [contributing codebase CODE-CB08](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Codebases%2FCODE-CB08)
// [task TASK-T12](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FTasks%2FTASK-T12)
// [task TASK-T14](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FTasks%2FTASK-T14)
// [task TASK-T15](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FTasks%2FTASK-T15)
// [task TASK-T17](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FTasks%2FTASK-T17)
// [task TASK-T18](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FTasks%2FTASK-T18)
// [task TASK-T19](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FTasks%2FTASK-T19)
// [task TASK-T20](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FTasks%2FTASK-T20)
// [task TASK-T21](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FTasks%2FTASK-T21)
// [task TASK-T22](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FTasks%2FTASK-T22)
// [task TASK-T23](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FTasks%2FTASK-T23)
// [task TASK-T24](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FTasks%2FTASK-T24)
// [task TASK-T25](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FTasks%2FTASK-T25)
// [task TASK-T26](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FTasks%2FTASK-T26)
// [task TASK-T27](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FTasks%2FTASK-T27)
// [separate reference example EX-numerical](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Exemplars%2FEX-numerical)
// [flow FLOW-F12](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Flows%2FFLOW-F12)
// [flow FLOW-F13](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Flows%2FFLOW-F13)
// [handbook HB-compatibility-map](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Handbook%2FSections%2FHB-compatibility-map)
// [handbook HB-identity-map](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Handbook%2FSections%2FHB-identity-map)
// [handbook HB-socket-map](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Handbook%2FSections%2FHB-socket-map)
// [API API-API01](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Interfaces%2FAPI%2FAPI-API01)
// [API API-API05](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Interfaces%2FAPI%2FAPI-API05)
// [API API-API09](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Interfaces%2FAPI%2FAPI-API09)
// [API API-API10](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Interfaces%2FAPI%2FAPI-API10)
// [action ACT-analysis.get](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Interfaces%2FActions%2FACT-analysis.get)
// [action ACT-analysis.request](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Interfaces%2FActions%2FACT-analysis.request)
// [IPC IPC-IPC01](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Interfaces%2FIPC%2FIPC-IPC01)
// [IPC IPC-IPC04](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Interfaces%2FIPC%2FIPC-IPC04)
// [IPC IPC-IPC05](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Interfaces%2FIPC%2FIPC-IPC05)
// [IPC IPC-IPC07](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Interfaces%2FIPC%2FIPC-IPC07)
// [public interface convention Module Public Contracts](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Interfaces%2FModule%20Public%20Contracts)
// [planned module MOD-numerical](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Modules%2FMOD-numerical)
// [plan SEC-api-sockets](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Plan%2FSEC-api-sockets)
// [plan SEC-economy](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Plan%2FSEC-economy)
// [plan SEC-module-design](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Plan%2FSEC-module-design)
// [plan SEC-numerical](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Plan%2FSEC-numerical)
// [plan SEC-security](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Plan%2FSEC-security)
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
// [schematic SC-SC01](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Schematics%2FSC-SC01)
// [schematic SC-SC02](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Schematics%2FSC-SC02)
// [schematic SC-SC03](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Schematics%2FSC-SC03)
// [schematic SC-SC10](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Schematics%2FSC-SC10)
// [schematic SC-SC13](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Schematics%2FSC-SC13)
// [schematic SC-SC14](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Schematics%2FSC-SC14)
// [schematic SC-SC15](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Schematics%2FSC-SC15)
// [schematic SC-SC17](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Schematics%2FSC-SC17)
// [schematic SC-SC18](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Schematics%2FSC-SC18)
// [schematic SC-SC19](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Schematics%2FSC-SC19)
// [schematic SC-SC22](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Schematics%2FSC-SC22)
// [source SRC-A02](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Sources%2FSRC-A02)
// [source SRC-S13](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Sources%2FSRC-S13)
// [testing standard Module Testing](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Standards%2FModule%20Testing)
// [progressive context workflow Module Context](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Workflows%2FModule%20Context)
// [module context scout CTX-numerical](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Context%2FModules%2FCTX-numerical)
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
// [adopted readiness convention Index](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FReadiness%2FIndex)
// [readiness criterion cluster F1](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FReadiness%2FF1)
// [readiness criterion cluster F2](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FReadiness%2FF2)
// [readiness criterion cluster F3](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FReadiness%2FF3)
// [readiness criterion cluster F4](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FReadiness%2FF4)
// [readiness criterion cluster F5](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FReadiness%2FF5)
// [readiness criterion cluster F7](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FReadiness%2FF7)
// [readiness improvement grouping R90-01](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FReadiness%2FR90-01)
// [readiness improvement grouping R90-03](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FReadiness%2FR90-03)
// [readiness improvement grouping R90-07](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FReadiness%2FR90-07)
// [readiness improvement grouping R90-09](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FReadiness%2FR90-09)
// [readiness improvement grouping R90-10](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FReadiness%2FR90-10)
// [Graphify corpus projection Index](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Graphify%2FIndex)
// [defensive security convention Daybreak Profile](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Security%2FDaybreak%20Profile)
// [defensive security convention RB05](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Operations%2FRunbooks%2FRB05)
// [defensive security convention Configuration](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Operations%2FConfiguration)
// [completion and operational convention Module Completion](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Standards%2FModule%20Completion)
// [completion and operational convention DONE-numerical](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Standards%2FCompletion%2FDONE-numerical)
// [completion and operational convention Context Handoff](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Context%20Handoff)
// [completion and operational convention Justfiles and Runbooks](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Operations%2FJustfiles%20and%20Runbooks)
// [completion and operational convention RB03](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Operations%2FRunbooks%2FRB03)
// [completion and operational convention RB04](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Operations%2FRunbooks%2FRB04)
// [applied learning LRN03](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Learnings%2FLRN03)
// [applied learning LRN05](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Learnings%2FLRN05)
// [applied learning LRN06](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Learnings%2FLRN06)
// [applied learning LRN08](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Learnings%2FLRN08)
// [applied learning LRN09](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Learnings%2FLRN09)
// [applied learning LRN10](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Learnings%2FLRN10)
// [applied learning LRN12](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Learnings%2FLRN12)
// [diary evidence source DR01](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Learnings%2FSources%2FDR01)
// [diary evidence source DR02](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Learnings%2FSources%2FDR02)
// [diary evidence source DR03](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Learnings%2FSources%2FDR03)
// [diary evidence source DR04](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Learnings%2FSources%2FDR04)
// [diary evidence source DR05](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Learnings%2FSources%2FDR05)
// [diary evidence source DR06](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Learnings%2FSources%2FDR06)
// [diary evidence source DR07](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Learnings%2FSources%2FDR07)
// [diary evidence source DR08](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Learnings%2FSources%2FDR08)
// [diary evidence source DR09](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Learnings%2FSources%2FDR09)
// [diary evidence source DR10](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Learnings%2FSources%2FDR10)
// [diary evidence source DR11](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Learnings%2FSources%2FDR11)
// [diary evidence source DR13](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Learnings%2FSources%2FDR13)
// Applicable learning IDs: LRN03, LRN05, LRN06, LRN08, LRN09, LRN10, LRN12; guidance only, engine detectors unqualified.
// [Assertions Measured Against the Code](obsidian://open?vault=my-diary.vault&file=Reflections%2FAssertions%20Measured%20Against%20the%20Code)
// [Mistakes I Made](obsidian://open?vault=my-diary.vault&file=Reflections%2FMistakes%20I%20Made)
// [The Antipattern Registers](obsidian://open?vault=my-diary.vault&file=Reflections%2FThe%20Antipattern%20Registers)
// [The Corpus of Me](obsidian://open?vault=my-diary.vault&file=Reflections%2FThe%20Corpus%20of%20Me)
// [The Seven Traits, Tested](obsidian://open?vault=my-diary.vault&file=Reflections%2FThe%20Seven%20Traits%2C%20Tested)
// [The Spellbook and the Ember](obsidian://open?vault=my-diary.vault&file=Reflections%2FThe%20Spellbook%20and%20the%20Ember)
// [Thematic Analysis of the Vaults](obsidian://open?vault=my-diary.vault&file=Reflections%2FThematic%20Analysis%20of%20the%20Vaults)
// [What My Ancestors Knew That I Did Not](obsidian://open?vault=my-diary.vault&file=Reflections%2FWhat%20My%20Ancestors%20Knew%20That%20I%20Did%20Not)
// [What Prototyping Is For](obsidian://open?vault=my-diary.vault&file=Reflections%2FWhat%20Prototyping%20Is%20For)
// [What the Workflow Is Worth](obsidian://open?vault=my-diary.vault&file=Reflections%2FWhat%20the%20Workflow%20Is%20Worth)
// [Why I Stopped Trusting Green](obsidian://open?vault=my-diary.vault&file=Reflections%2FWhy%20I%20Stopped%20Trusting%20Green)
// [Working in Sandboxes on Kinoite](obsidian://open?vault=my-diary.vault&file=Reflections%2FWorking%20in%20Sandboxes%20on%20Kinoite)
// HEE3-ANCHORS-END

//! Pure HEE3-Analysis/1 validation. Results are advisory and cannot mutate owners.
//! Process custody and application action integration are separate obligations.
#![forbid(missing_docs)]

use crate::contracts::{Sha256Digest, UuidV4, parse_u64_decimal};
use serde::{Deserialize, Deserializer, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

/// Largest request body, in bytes, [`Dataset::decode`] accepts; refused before parsing.
pub const MAX_REQUEST: usize = 1_048_576;
/// Largest report or refusal body, in bytes, accepted from the evaluator; also the
/// stream limit [`process::analyze`] gives the child.
pub const MAX_REPORT: usize = 65_536;
/// Largest number of observation rows one request may carry.
pub const MAX_ROWS: usize = 4096;
const HEX: &[u8; 16] = b"0123456789abcdef";

/// Why a request, report or refusal body was refused. Every refusal is final for
/// those bytes; none is a partial acceptance.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Invalid {
    /// A size bound: an empty or over-limit body, nesting deeper than 32, or a row
    /// count that is zero, above [`MAX_ROWS`] or different from `shape.rows`.
    Bound,
    /// Not exactly one JSON object of the closed shape: malformed or trailing text,
    /// a duplicate, unknown or missing key, a wrong type, or an unknown code.
    Encoding,
    /// A request, task or attempt ID that is not a UUID v4, a malformed artifact
    /// digest or generation, or two rows with one attempt ID.
    Identity,
    /// A fixed value other than v1's: protocol, version, recipe, units, field
    /// count, a non-decimal cutoff or expiry, or a refusal's envelope.
    Schema,
    /// A row value outside its domain: a non-finite or out-of-range `elapsed_ms`,
    /// `censored` disagreeing with `running`, or usage that is not a U32 decimal.
    Domain,
    /// The cutoff is after, or the expiry at or before, the supplied time.
    Stale,
    /// A report or refusal that names another request: a different digest, ID,
    /// subject, recipe, cutoff, expiry or units.
    Binding,
    /// A report whose counts differ from, or whose fraction or mean is non-finite or
    /// outside the declared tolerance of, the independent Rust reference.
    Statistics,
}

/// Requires the key to be present; `null` decodes to `None`, never a default.
fn nullable<'de, D: Deserializer<'de>, T: Deserialize<'de>>(d: D) -> Result<Option<T>, D::Error> {
    Option::deserialize(d)
}

/// Stable refusal code the fixed Julia entrypoint (`julia/bin/analysis.jl`) writes
/// before exiting with status 2. The set is closed: it is exactly the codes that
/// the entrypoint and `analyze` in `julia/src/Evaluate.jl` can raise.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum JuliaCode {
    /// Julia refused a size bound (input or output over its limit, or row bounds).
    Bound,
    /// Julia refused a row value outside its domain.
    Domain,
    /// Julia refused a duplicate decoded key.
    Duplicate,
    /// Julia refused the bytes as malformed JSON or number spelling.
    Encoding,
    /// Julia refused an identifier or digest.
    Identity,
    /// Julia refused a fixed protocol, recipe, unit or shape value.
    Schema,
    /// Julia refused the request as outside its cutoff/expiry window at receipt.
    Stale,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
/// The task attempt and artifact a dataset describes; echoed exactly by a report.
pub struct Subject {
    /// Task ID, a UUID v4.
    pub task_id: String,
    /// Attempt ID, a UUID v4.
    pub attempt_id: String,
    /// Attempt generation, a canonical U64 decimal string.
    pub generation: String,
    /// Artifact digest, `sha256:` and 64 lowercase hex digits.
    pub artifact_sha256: String,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
/// The evaluation recipe; v1 accepts only `descriptive` version 1.
pub struct Recipe {
    /// Recipe name; only `descriptive` is admitted.
    pub id: String,
    /// Recipe version; only 1 is admitted.
    pub version: u32,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
/// Units of the observation columns; v1 accepts only `ms` and `token`.
pub struct Units {
    /// Unit of `elapsed_ms`; only `ms`.
    pub elapsed: String,
    /// Unit of `usage_tokens`; only `token`.
    pub usage: String,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
/// Declared table shape, checked against the rows actually received.
pub struct Shape {
    /// Row count; must equal the number of observations.
    pub rows: u32,
    /// Fields per row; only 5.
    pub fields: u32,
}
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// An attempt's outcome at the dataset cutoff.
pub enum Outcome {
    /// Finished and accepted.
    Accepted,
    /// Finished and failed.
    Failed,
    /// Cancelled before finishing.
    Cancelled,
    /// Abandoned without a settled result.
    Abandoned,
    /// Still running at the cutoff; the only right-censored outcome.
    Running,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
/// One attempt row of a dataset.
pub struct Observation {
    /// Attempt ID, a UUID v4, unique within the dataset.
    pub attempt_id: String,
    /// Outcome at the cutoff.
    pub outcome: Outcome,
    /// Observed elapsed time, finite and within `[0, 86_400_000]` ms; for a
    /// running attempt, exposure through the cutoff.
    pub elapsed_ms: f64,
    /// Known token usage as a U32 decimal; `None` (JSON `null`, never omitted) is
    /// unknown, which differs from a known `"0"`.
    #[serde(deserialize_with = "nullable")]
    pub usage_tokens: Option<String>,
    /// True exactly when `outcome` is `Running`.
    pub censored: bool,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
/// A decoded HEE3-Analysis/1 request, the closed wire shape of a dataset.
pub struct Request {
    /// Protocol name; only `hee3.analysis`.
    pub protocol: String,
    /// Protocol version; only 1.
    pub version: u32,
    /// Request ID, a UUID v4.
    pub request_id: String,
    /// The attempt and artifact described.
    pub subject: Subject,
    /// Immutable cutoff, U64 decimal Unix milliseconds; never in the future.
    pub cutoff_unix_ms: String,
    /// Expiry, U64 decimal Unix milliseconds; must still be in the future.
    pub expires_unix_ms: String,
    /// Evaluation recipe.
    pub recipe: Recipe,
    /// Column units.
    pub units: Units,
    /// Declared shape.
    pub shape: Shape,
    /// Every attempt at the cutoff, in input order; 1 to [`MAX_ROWS`] rows.
    pub observations: Vec<Observation>,
}
/// Canonical decimal strings, including zero, distinguish unknown usage from zero.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Counts {
    /// All rows.
    pub total: String,
    /// Rows with outcome `accepted`.
    pub accepted: String,
    /// Rows with outcome `failed`.
    pub failed: String,
    /// Rows with outcome `cancelled`.
    pub cancelled: String,
    /// Rows with outcome `abandoned`.
    pub abandoned: String,
    /// Rows with outcome `running`.
    pub running: String,
    /// Rows whose usage is unknown (`null`).
    pub unknown_usage: String,
    /// Right-censored rows (equal to `running` in v1).
    pub censored: String,
    /// Sum of all known usage values.
    pub known_usage_sum: String,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
/// A descriptive/1 report: the request's identity echoed exactly, plus its
/// statistics. Advisory only; accepting one grants no task or policy change.
pub struct Report {
    /// Echoed protocol name.
    pub protocol: String,
    /// Echoed protocol version.
    pub version: u32,
    /// Echoed request ID.
    pub request_id: String,
    /// Digest of the exact request bytes, `sha256:` and 64 lowercase hex digits.
    pub request_sha256: String,
    /// Echoed subject.
    pub subject: Subject,
    /// Echoed cutoff.
    pub cutoff_unix_ms: String,
    /// Echoed expiry.
    pub expires_unix_ms: String,
    /// Echoed recipe.
    pub recipe: Recipe,
    /// Echoed units.
    pub units: Units,
    /// Exact outcome and usage counts.
    pub counts: Counts,
    /// `accepted / total`; within `1e-12` of the Rust reference.
    pub acceptance_fraction: f64,
    /// Compensated mean of `elapsed_ms` over input order; within
    /// `8 * f64::EPSILON * max(|expected|, 1)` of the Rust reference.
    pub mean_observed_ms: f64,
}
/// Wire form written by `julia/bin/analysis.jl` before `exit(2)`; every key required.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct JuliaError {
    protocol: String,
    version: u32,
    kind: String,
    #[serde(deserialize_with = "nullable")]
    request_sha256: Option<String>,
    #[serde(deserialize_with = "nullable")]
    binding: Option<Binding>,
    code: JuliaCode,
    diagnostic: String,
}
/// Request header Julia echoes when it could decode one.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Binding {
    request_id: String,
    subject: Subject,
    recipe: Recipe,
    cutoff_unix_ms: String,
    expires_unix_ms: String,
    units: Units,
}
/// Owns exact received bytes and validated fields without exposing mutable access.
///
/// ```
/// use habitat_engine::numerical::{Dataset, Invalid};
/// # fn main() -> Result<(), Invalid> {
/// let raw = include_bytes!("../tests/fixtures/t21/J01.json");
/// // Rows: accepted/10 ms/4 tokens, failed/20/unknown, cancelled/30/0,
/// // abandoned/40/unknown, running/50/unknown; valid only inside its window.
/// let dataset = Dataset::decode(raw, 1_769_999_995_000)?;
/// let report = dataset.reference()?;
/// assert_eq!(report.request_sha256, dataset.digest());
/// assert_eq!(report.counts.total, "5");
/// assert_eq!(report.counts.accepted, "1");
/// assert_eq!(report.counts.unknown_usage, "3");
/// assert_eq!(report.counts.known_usage_sum, "4");
/// assert_eq!(report.acceptance_fraction.to_bits(), 0.2_f64.to_bits());
/// assert_eq!(report.mean_observed_ms.to_bits(), 30.0_f64.to_bits());
/// // At its expiry the same bytes are refused, so no report can be built.
/// assert_eq!(Dataset::decode(raw, 1_770_000_000_000).err(), Some(Invalid::Stale));
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct Dataset {
    raw: Vec<u8>,
    request: Request,
    digest: String,
}

impl Dataset {
    /// Decode one bounded EOF object, including duplicate and unknown-key refusal.
    /// # Errors
    /// Returns explicit schema, identity, domain, time or resource refusal.
    pub fn decode(raw: &[u8], now_ms: u64) -> Result<Self, Invalid> {
        let request: Request = decode(raw, MAX_REQUEST)?;
        validate(&request, now_ms)?;
        let mut digest = String::from("sha256:");
        for byte in Sha256::digest(raw) {
            digest.push(char::from(HEX[usize::from(byte >> 4)]));
            digest.push(char::from(HEX[usize::from(byte & 15)]));
        }
        Ok(Self {
            raw: raw.to_vec(),
            request,
            digest,
        })
    }
    /// The exact bytes received, as sent to the evaluator; never re-serialized.
    #[must_use]
    pub fn raw(&self) -> &[u8] {
        &self.raw
    }
    /// The validated decoded request.
    #[must_use]
    pub fn request(&self) -> &Request {
        &self.request
    }
    /// Digest of [`Dataset::raw`], `sha256:` and 64 lowercase hex digits; the
    /// binding every report and refusal must echo.
    #[must_use]
    pub fn digest(&self) -> &str {
        &self.digest
    }

    /// Recompute the descriptive reference over every row in its original order.
    /// # Errors
    /// Refuses any violated internal scalar invariant.
    pub fn reference(&self) -> Result<Report, Invalid> {
        let q = &self.request;
        let mut counts = [0_u32; 5];
        let mut unknown = 0_u64;
        let mut usage = 0_u64;
        let mut sum = 0.0;
        let mut correction = 0.0;
        for row in &q.observations {
            counts[match row.outcome {
                Outcome::Accepted => 0,
                Outcome::Failed => 1,
                Outcome::Cancelled => 2,
                Outcome::Abandoned => 3,
                Outcome::Running => 4,
            }] += 1;
            if let Some(value) = &row.usage_tokens {
                // The immutable validated dataset bounds each value to u32.
                usage += parse_u64_decimal(value).map_err(|_| Invalid::Domain)?;
            } else {
                unknown += 1;
            }
            let adjusted = row.elapsed_ms - correction;
            let next = sum + adjusted;
            correction = (next - sum) - adjusted;
            sum = next;
        }
        Ok(Report {
            protocol: q.protocol.clone(),
            version: q.version,
            request_id: q.request_id.clone(),
            request_sha256: self.digest.clone(),
            subject: q.subject.clone(),
            cutoff_unix_ms: q.cutoff_unix_ms.clone(),
            expires_unix_ms: q.expires_unix_ms.clone(),
            recipe: q.recipe.clone(),
            units: q.units.clone(),
            counts: Counts {
                total: q.shape.rows.to_string(),
                accepted: counts[0].to_string(),
                failed: counts[1].to_string(),
                cancelled: counts[2].to_string(),
                abandoned: counts[3].to_string(),
                running: counts[4].to_string(),
                unknown_usage: unknown.to_string(),
                censored: counts[4].to_string(),
                known_usage_sum: usage.to_string(),
            },
            acceptance_fraction: f64::from(counts[0]) / f64::from(q.shape.rows),
            mean_observed_ms: sum / f64::from(q.shape.rows),
        })
    }

    /// Decode the fixed Julia entrypoint's bounded error object as a refusal of
    /// exactly this dataset. The digest must name these bytes; an echoed request
    /// binding, when Julia could decode one, must equal this request's.
    /// # Errors
    /// `Bound` or `Encoding` for an oversize, malformed, unknown-field or
    /// unknown-code body (including a success report); `Schema` for another
    /// protocol, version, kind or diagnostic; `Binding` for an absent or other
    /// request digest or a different echoed binding.
    pub fn refusal(&self, raw: &[u8]) -> Result<JuliaCode, Invalid> {
        let e: JuliaError = decode(raw, MAX_REPORT)?;
        if e.protocol != "hee3.analysis"
            || e.version != 1
            || e.kind != "error"
            || e.diagnostic != "analysis refusal"
        {
            return Err(Invalid::Schema);
        }
        if e.request_sha256.as_deref() != Some(self.digest.as_str()) {
            return Err(Invalid::Binding);
        }
        if let Some(b) = e.binding {
            let q = &self.request;
            if b.request_id != q.request_id
                || b.subject != q.subject
                || b.recipe != q.recipe
                || b.cutoff_unix_ms != q.cutoff_unix_ms
                || b.expires_unix_ms != q.expires_unix_ms
                || b.units != q.units
            {
                return Err(Invalid::Binding);
            }
        }
        Ok(e.code)
    }

    /// Validate a received report independently against the original dataset.
    /// # Errors
    /// Refuses malformed, stale, mismatched or numerically incorrect reports.
    pub fn report(&self, raw: &[u8], now_ms: u64) -> Result<Report, Invalid> {
        validate(&self.request, now_ms)?;
        let r: Report = decode(raw, MAX_REPORT)?;
        let expected = self.reference()?;
        if r.protocol != expected.protocol
            || r.version != expected.version
            || r.request_id != expected.request_id
            || r.request_sha256 != expected.request_sha256
            || r.subject != expected.subject
            || r.cutoff_unix_ms != expected.cutoff_unix_ms
            || r.expires_unix_ms != expected.expires_unix_ms
            || r.recipe != expected.recipe
            || r.units != expected.units
        {
            return Err(Invalid::Binding);
        }
        if r.counts != expected.counts
            || !r.acceptance_fraction.is_finite()
            || !r.mean_observed_ms.is_finite()
            || (r.acceptance_fraction - expected.acceptance_fraction).abs() > 1e-12
            || (r.mean_observed_ms - expected.mean_observed_ms).abs()
                > 8.0 * f64::EPSILON * expected.mean_observed_ms.abs().max(1.0)
        {
            return Err(Invalid::Statistics);
        }
        Ok(r)
    }
}

fn decode<T: serde::de::DeserializeOwned>(raw: &[u8], cap: usize) -> Result<T, Invalid> {
    if raw.is_empty() || raw.len() > cap {
        return Err(Invalid::Bound);
    }
    // Limit nesting before serde allocation. Typed structs independently enforce
    // decoded-key uniqueness, closed fields and the complete accepted shape.
    let mut depth = 0_u32;
    let mut quoted = false;
    let mut escaped = false;
    for &b in raw {
        if quoted {
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == b'"' {
                quoted = false;
            }
        } else if b == b'"' {
            quoted = true;
        } else if b == b'{' || b == b'[' {
            depth += 1;
            if depth > 32 {
                return Err(Invalid::Bound);
            }
        } else if b == b'}' || b == b']' {
            depth = depth.checked_sub(1).ok_or(Invalid::Encoding)?;
        }
    }
    serde_json::from_slice(raw).map_err(|_| Invalid::Encoding)
}

fn validate(q: &Request, now: u64) -> Result<(), Invalid> {
    if q.protocol != "hee3.analysis"
        || q.version != 1
        || q.recipe.id != "descriptive"
        || q.recipe.version != 1
        || q.units.elapsed != "ms"
        || q.units.usage != "token"
        || q.shape.fields != 5
    {
        return Err(Invalid::Schema);
    }
    for id in [&q.request_id, &q.subject.task_id, &q.subject.attempt_id] {
        UuidV4::parse(id).map_err(|_| Invalid::Identity)?;
    }
    Sha256Digest::parse(&q.subject.artifact_sha256).map_err(|_| Invalid::Identity)?;
    parse_u64_decimal(&q.subject.generation).map_err(|_| Invalid::Identity)?;
    let cutoff = parse_u64_decimal(&q.cutoff_unix_ms).map_err(|_| Invalid::Schema)?;
    let expiry = parse_u64_decimal(&q.expires_unix_ms).map_err(|_| Invalid::Schema)?;
    if cutoff > now || expiry <= now {
        return Err(Invalid::Stale);
    }
    if q.observations.is_empty()
        || q.observations.len() > MAX_ROWS
        || usize::try_from(q.shape.rows).ok() != Some(q.observations.len())
    {
        return Err(Invalid::Bound);
    }
    let mut ids = BTreeSet::new();
    for row in &q.observations {
        UuidV4::parse(&row.attempt_id).map_err(|_| Invalid::Identity)?;
        if !ids.insert(&row.attempt_id) {
            return Err(Invalid::Identity);
        }
        if !row.elapsed_ms.is_finite()
            || !(0.0..=86_400_000.0).contains(&row.elapsed_ms)
            || row.censored != (row.outcome == Outcome::Running)
        {
            return Err(Invalid::Domain);
        }
        if let Some(value) = &row.usage_tokens {
            let value = parse_u64_decimal(value).map_err(|_| Invalid::Domain)?;
            u32::try_from(value).map_err(|_| Invalid::Domain)?;
        }
    }
    Ok(())
}

/// Bounded advisory Julia process exchange, without task/grant authority.
pub mod process;
