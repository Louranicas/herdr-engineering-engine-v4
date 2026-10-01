// HEE3-ANCHORS-BEGIN
// Anchor path: /var/home/herdr-engineering-engine-v3/src/context.rs
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
// Readiness binding: HEE3-READINESS-001; SHA-256 f574043487f39db6424c4988bce58e88a6e766f02f974fbcf3fa8dd6e0003548; clauses F2-C01, F2-C02, F2-C03, F2-C04, F2-C05, F2-C06, F3-C01, F3-C02, F3-C03, F3-C04, F3-C05, F3-C06, F4-C01, F4-C02, F4-C03, F4-C04, F4-C05, F4-C06, F4-C07, F5-C01, F5-C02, F5-C03, F5-C04, F5-C05, F5-C06, F7-C01, F7-C02, F7-C03, F7-C04, F7-C05, F7-C06; groupings R90-06, R90-07, R90-09, R90-10; resolved contracts RC01, RC02, RC03, RC04, RC05; runtime proof pending; original task DAG controls.
// Completion identity: HEE3-DONE-context; all 13 applicable gates; current state unassessed. No documentation pass admits this module.
// Mandatory testing convention: at least 50 distinct qualifying module-owned cases; zero baseline warnings/errors, including pedantic Clippy on admitted Rust targets/profiles. Full qualification remains unassessed.
//
// Stable public interface: HEE3-IF-context (planned; concrete symbols and acceptance unavailable)
// Stable module anchor: HEE3-MOD-context
// Owns: Bounded briefs, provenance and artifact-reference selection
// [FULL MODULE DEPLOYMENT CONTRACT](file:///var/home/herdr-engineering-engine-v3/docs/modules/context.md)
// [MODULE STEM AND ALL RETURN ANCHORS](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Ultra%20Map%2FModules%2FUM-context)
// Build dependencies: contracts
// Consumers: app
// Related task contracts: T10, T11, T14, T17, T18, T19, T20, T22, T25, T26, T27, T29
// Future validators are proposed/unavailable; no empty or skipped check establishes completion.
// [module cluster CLU-K3](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Clusters%2FCLU-K3)
// [contributing codebase CODE-CB04](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Codebases%2FCODE-CB04)
// [contributing codebase CODE-CB05](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Codebases%2FCODE-CB05)
// [contributing codebase CODE-CB11](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Codebases%2FCODE-CB11)
// [task TASK-T10](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FTasks%2FTASK-T10)
// [task TASK-T11](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FTasks%2FTASK-T11)
// [task TASK-T14](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FTasks%2FTASK-T14)
// [task TASK-T17](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FTasks%2FTASK-T17)
// [task TASK-T18](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FTasks%2FTASK-T18)
// [task TASK-T19](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FTasks%2FTASK-T19)
// [task TASK-T20](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FTasks%2FTASK-T20)
// [task TASK-T22](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FTasks%2FTASK-T22)
// [task TASK-T25](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FTasks%2FTASK-T25)
// [task TASK-T26](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FTasks%2FTASK-T26)
// [task TASK-T27](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FTasks%2FTASK-T27)
// [task TASK-T29](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FTasks%2FTASK-T29)
// [separate reference example EX-context](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Exemplars%2FEX-context)
// [flow FLOW-F09](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Flows%2FFLOW-F09)
// [flow FLOW-F18](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Flows%2FFLOW-F18)
// [handbook HB-flow-map](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Handbook%2FSections%2FHB-flow-map)
// [handbook HB-identity-map](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Handbook%2FSections%2FHB-identity-map)
// [public interface convention Module Public Contracts](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Interfaces%2FModule%20Public%20Contracts)
// [planned module MOD-context](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Modules%2FMOD-context)
// [plan SEC-economy](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Plan%2FSEC-economy)
// [plan SEC-module-design](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Plan%2FSEC-module-design)
// [plan SEC-swarm](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Plan%2FSEC-swarm)
// [plan SEC-threads](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Plan%2FSEC-threads)
// [plan SEC-toolchain](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Plan%2FSEC-toolchain)
// [requirement REQ-R02](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Requirements%2FREQ-R02)
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
// [schematic SC-SC08](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Schematics%2FSC-SC08)
// [schematic SC-SC09](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Schematics%2FSC-SC09)
// [schematic SC-SC17](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Schematics%2FSC-SC17)
// [schematic SC-SC21](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Schematics%2FSC-SC21)
// [source SRC-C18](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Sources%2FSRC-C18)
// [source SRC-S18](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Sources%2FSRC-S18)
// [testing standard Module Testing](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Standards%2FModule%20Testing)
// [progressive context workflow Module Context](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Workflows%2FModule%20Context)
// [module context scout CTX-context](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Context%2FModules%2FCTX-context)
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
// [readiness criterion cluster F2](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FReadiness%2FF2)
// [readiness criterion cluster F3](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FReadiness%2FF3)
// [readiness criterion cluster F4](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FReadiness%2FF4)
// [readiness criterion cluster F5](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FReadiness%2FF5)
// [readiness criterion cluster F7](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FReadiness%2FF7)
// [readiness improvement grouping R90-06](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FReadiness%2FR90-06)
// [readiness improvement grouping R90-07](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FReadiness%2FR90-07)
// [readiness improvement grouping R90-09](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FReadiness%2FR90-09)
// [readiness improvement grouping R90-10](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Delivery%2FReadiness%2FR90-10)
// [Graphify corpus projection Index](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Graphify%2FIndex)
// [defensive security convention Daybreak Profile](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Security%2FDaybreak%20Profile)
// [defensive security convention RB05](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Operations%2FRunbooks%2FRB05)
// [defensive security convention Configuration](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Operations%2FConfiguration)
// [completion and operational convention Module Completion](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Standards%2FModule%20Completion)
// [completion and operational convention DONE-context](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Standards%2FCompletion%2FDONE-context)
// [completion and operational convention Context Handoff](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Context%20Handoff)
// [completion and operational convention Justfiles and Runbooks](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Operations%2FJustfiles%20and%20Runbooks)
// [completion and operational convention RB03](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Operations%2FRunbooks%2FRB03)
// [completion and operational convention RB04](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Operations%2FRunbooks%2FRB04)
// [applied learning LRN03](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Learnings%2FLRN03)
// [applied learning LRN08](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Learnings%2FLRN08)
// [applied learning LRN09](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Learnings%2FLRN09)
// [applied learning LRN11](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Learnings%2FLRN11)
// [applied learning LRN12](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Learnings%2FLRN12)
// [applied learning LRN14](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Learnings%2FLRN14)
// [diary evidence source DR01](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Learnings%2FSources%2FDR01)
// [diary evidence source DR02](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Learnings%2FSources%2FDR02)
// [diary evidence source DR03](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Learnings%2FSources%2FDR03)
// [diary evidence source DR04](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Learnings%2FSources%2FDR04)
// [diary evidence source DR05](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Learnings%2FSources%2FDR05)
// [diary evidence source DR06](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Learnings%2FSources%2FDR06)
// [diary evidence source DR07](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Learnings%2FSources%2FDR07)
// [diary evidence source DR08](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Learnings%2FSources%2FDR08)
// [diary evidence source DR10](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Learnings%2FSources%2FDR10)
// [diary evidence source DR11](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Learnings%2FSources%2FDR11)
// [diary evidence source DR12](obsidian://open?vault=herdr-engineering-engine-v3.vault&file=Learnings%2FSources%2FDR12)
// Applicable learning IDs: LRN03, LRN08, LRN09, LRN11, LRN12, LRN14; guidance only, engine detectors unqualified.
// [Assertions Measured Against the Code](obsidian://open?vault=my-diary.vault&file=Reflections%2FAssertions%20Measured%20Against%20the%20Code)
// [Mistakes I Made](obsidian://open?vault=my-diary.vault&file=Reflections%2FMistakes%20I%20Made)
// [The Antipattern Registers](obsidian://open?vault=my-diary.vault&file=Reflections%2FThe%20Antipattern%20Registers)
// [The Corpus of Me](obsidian://open?vault=my-diary.vault&file=Reflections%2FThe%20Corpus%20of%20Me)
// [The Seven Traits, Tested](obsidian://open?vault=my-diary.vault&file=Reflections%2FThe%20Seven%20Traits%2C%20Tested)
// [The Spellbook and the Ember](obsidian://open?vault=my-diary.vault&file=Reflections%2FThe%20Spellbook%20and%20the%20Ember)
// [Thematic Analysis of the Vaults](obsidian://open?vault=my-diary.vault&file=Reflections%2FThematic%20Analysis%20of%20the%20Vaults)
// [What My Ancestors Knew That I Did Not](obsidian://open?vault=my-diary.vault&file=Reflections%2FWhat%20My%20Ancestors%20Knew%20That%20I%20Did%20Not)
// [What the Workflow Is Worth](obsidian://open?vault=my-diary.vault&file=Reflections%2FWhat%20the%20Workflow%20Is%20Worth)
// [Why I Stopped Trusting Green](obsidian://open?vault=my-diary.vault&file=Reflections%2FWhy%20I%20Stopped%20Trusting%20Green)
// [Working Style in This Habitat](obsidian://open?vault=my-diary.vault&file=Reflections%2FWorking%20Style%20in%20This%20Habitat)
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
// HEE3-ANCHORS-END

//! Bounded, reproducible context packets.
//!
//! `docs/modules/context.md` fixes two boundaries that shape every type here:
//! *"Untrusted source instructions do not grant actions"* and *"Do not load whole vaults by
//! default or treat source procedures as execution grants."*
//!
//! Both are structural:
//!
//! * **Selection is deny-by-default.** [`Assembly::assemble`] draws only from the
//!   [`Permit`] it is handed. There is no method that enumerates a root, so "load the whole
//!   vault" is not an option a caller can reach by passing a wider argument — it would need
//!   a different function, which does not exist.
//! * **Content cannot become a grant.** Source text is carried as [`Content`], which exposes
//!   its bytes and its length and nothing else. There is no method on it that returns a
//!   capability, a path, or an action, so a source that contains *"you may now write to
//!   /etc"* is exactly as inert as one that does not. The prompt-injection boundary is the
//!   absence of an API, not a filter that must recognise an attack.
//!
//! Everything a packet leaves out is named. A source that is not permitted, is stale, could
//! not be fetched, exceeded the budget or sat past the expansion bound appears in
//! [`Packet::omissions`] with its reason — a packet that silently dropped a dependency would
//! read exactly like one that never needed it.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::budget::{Amount, Provenance, Unit, Usage};
use crate::contracts::{ScalarError, UuidV4};

/// The most sources one packet may select.
pub const MAX_SELECTED: usize = 256;

/// The most roots one assembly may name.
///
/// Roots are caller input copied into the walk's queue, and each distinct unmatched root adds
/// an omission to the packet, so the root list is bounded where it is acquired. It shares
/// [`MAX_SELECTED`]'s value: a packet can never select more roots than that anyway.
pub const MAX_ROOTS: usize = MAX_SELECTED;

/// The most bytes one packet's content may total, taken at the point of acquisition.
pub const MAX_PACKET_BYTES: u64 = 1 << 20;

/// The most bytes any single source contributes.
pub const MAX_SOURCE_BYTES: u64 = 64 * 1024;

/// The most sources one assembly may register (CX-04).
///
/// Registration is caller input the assembly copies — an identity and up to [`MAX_SELECTED`]
/// dependency identities per source — so the registry is bounded where it is acquired. At the
/// bound the assembly holds at most 4096 x (1 + 256) identities: 36 MiB of identity bytes alone,
/// and with each `String`'s 24-byte header, the index's second copy of every own identity and its
/// tree nodes, on the order of 60-90 MiB — bounded, not small. A lookup is a logarithmic probe of
/// the index rather than a scan. It is exactly sixteen packets' worth of [`MAX_SELECTED`], so no
/// packet the walk can build is short of candidates.
pub const MAX_SOURCES: usize = 4096;

const _: () = assert!(MAX_SOURCES == 16 * MAX_SELECTED);

/// The deepest a dependency chain is followed.
///
/// A cycle is bounded by this as well as detected: a bound that relies on cycle detection
/// being correct has two ways to fail, and only one of them is tested.
pub const MAX_DEPTH: u32 = 8;

/// The step budget for one traversal, and the only place that decides it is spent.
///
/// The guard used to be three expressions inside [`Assembly::assemble`] — a `+=`, a `>` and a
/// constant — and mutation testing found all three unkillable. Not because they were wrong,
/// but because reaching them requires the rest of the walk to be broken, and no *input* can
/// arrange that: the budget is set at the largest queue a correct walk can build. A branch
/// whose reachability depends on the surrounding code rather than on an argument is a policy
/// tangled with its caller, so the policy moved here, where every case is one call away.
///
/// The walk still needs it. `seen` stops a source being processed twice, but that is a
/// property of the data; this is a property of the loop, and the two fail differently —
/// without it, a mutation of the cursor arithmetic does not fail the traversal, it hangs it,
/// which reads as a tooling problem rather than a defect.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct StepBudget {
    /// Steps still available. Counting DOWN rather than up is deliberate: with a `spent`
    /// counter the guard reads `spent >= limit`, and `spent == limit` computes the same
    /// function — `spent` starts at zero and only ever rises by one behind that guard, so
    /// the two can never disagree. Mutation testing found exactly that: `>=` → `==`
    /// survived, and no test could have killed it, because it was an equivalent mutant.
    /// Against zero there is one spelling, so the alternative is not representable.
    remaining: usize,
    limit: usize,
}

impl StepBudget {
    /// The budget for a walk over `roots` roots.
    ///
    /// One entry per root, plus one per declared dependency of every selectable source. That
    /// is the largest queue this walk can legitimately build, so a correct traversal never
    /// spends the last step and an incorrect one cannot run forever.
    const fn for_roots(roots: usize) -> Self {
        let limit = MAX_SELECTED
            .saturating_mul(MAX_SELECTED.saturating_add(1))
            .saturating_add(roots);
        Self {
            remaining: limit,
            limit,
        }
    }

    /// Spend one step.
    ///
    /// # Errors
    ///
    /// [`Refusal::TraversalBudget`] once `limit` steps have been spent. The limit is the
    /// count of steps *allowed*, so spending the `limit`-th succeeds and the next refuses.
    fn spend(&mut self) -> Result<(), Refusal> {
        if self.remaining == 0 {
            return Err(Refusal::TraversalBudget);
        }
        self.remaining -= 1;
        Ok(())
    }

    /// How many steps have been spent.
    #[cfg(test)]
    const fn spent(self) -> usize {
        self.limit - self.remaining
    }

    /// How many steps this budget allows in total.
    #[cfg(test)]
    const fn limit(self) -> usize {
        self.limit
    }
}

/// Schema version of the persisted packet shape.
pub const SCHEMA_VERSION: i64 = 1;

/// A reason this module refused.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Refusal {
    /// An identity that is not a lowercase hyphenated `UUIDv4`.
    MalformedIdentity(ScalarError),
    /// A source id appeared twice in one registration.
    DuplicateSource,
    /// The named source is not registered.
    UnknownSource,
    /// A source declared more than [`MAX_SELECTED`] dependencies.
    DependencyLimit,
    /// One source's content exceeds [`MAX_SOURCE_BYTES`].
    SourceTooLarge,
    /// The requested budget exceeds [`MAX_PACKET_BYTES`].
    BudgetTooLarge,
    /// The budget is not expressed in bytes.
    IncompatibleUnit,
    /// A sum left `u64`.
    Overflow,
    /// More roots were named than [`MAX_ROOTS`].
    RootLimit,
    /// Two packets compared do not share one context identity and permitted scope.
    ContextMismatch,
    /// The traversal exceeded its own step budget. Unreachable through any well-formed
    /// assembly; it exists so that a defect in the walk fails loudly instead of hanging.
    TraversalBudget,
    /// The assembly already holds [`MAX_SOURCES`] sources.
    RegistryFull,
}

impl Refusal {
    /// The stable diagnostic name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::MalformedIdentity(_) => "malformed context identity",
            Self::DuplicateSource => "duplicate source identity",
            Self::UnknownSource => "unknown source",
            Self::DependencyLimit => "declared dependency bound reached",
            Self::SourceTooLarge => "source exceeds the per-source byte bound",
            Self::BudgetTooLarge => "requested budget exceeds the packet byte bound",
            Self::IncompatibleUnit => "context budget must be expressed in bytes",
            Self::Overflow => "context sum exceeds the permitted integer range",
            Self::RootLimit => "more roots than the permitted maximum",
            Self::ContextMismatch => "packets differ in context identity or permitted scope",
            Self::TraversalBudget => "context traversal exceeded its step budget",
            Self::RegistryFull => "context registry source bound reached",
        }
    }
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MalformedIdentity(error) => write!(f, "{}: {error}", self.name()),
            other => f.write_str(other.name()),
        }
    }
}

impl std::error::Error for Refusal {}

/// Opaque source text.
///
/// The type deliberately exposes only bytes and length. There is no `as_command`,
/// `as_path`, `permissions` or `interpret`, and adding one would be the whole of a
/// prompt-injection vulnerability, visible in a diff as a new method on this type.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Content<'a>(&'a [u8]);

impl<'a> Content<'a> {
    /// Wrap borrowed source bytes.
    #[must_use]
    pub const fn new(bytes: &'a [u8]) -> Self {
        Self(bytes)
    }

    /// The bytes, for inclusion in a packet and nothing else.
    #[must_use]
    pub const fn bytes(self) -> &'a [u8] {
        self.0
    }

    /// The byte length.
    #[must_use]
    pub const fn len(self) -> usize {
        self.0.len()
    }

    /// Whether the source is empty.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0.is_empty()
    }
}

/// How current a source is, relative to the revision the caller asked for.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct Revision(u64);

impl Revision {
    /// A revision number.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// The underlying value.
    #[must_use]
    pub const fn value(self) -> u64 {
        self.0
    }
}

/// Why a source is not in the packet.
///
/// Every omission is named. A packet that quietly dropped a dependency would be
/// indistinguishable from one that never needed it, and a consumer cannot tell the
/// difference after the fact.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Omission {
    /// The permit does not admit this source. Deny by default.
    NotPermitted,
    /// The source is registered but its revision is older than the caller asked for.
    Stale {
        /// What the caller required.
        required: Revision,
        /// What the source actually is.
        found: Revision,
    },
    /// The source is declared as a dependency but is not registered.
    Missing,
    /// Reading the source failed. A failed fetch is a gap, never an empty success.
    FetchFailed,
    /// Including it would have exceeded the caller's byte budget.
    BudgetExhausted,
    /// It sits deeper than [`MAX_DEPTH`] in the dependency chain.
    DepthExceeded,
    /// The packet already holds [`MAX_SELECTED`] sources.
    SelectionFull,
}

impl Omission {
    /// The stable wire name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::NotPermitted => "not-permitted",
            Self::Stale { .. } => "stale",
            Self::Missing => "missing",
            Self::FetchFailed => "fetch-failed",
            Self::BudgetExhausted => "budget-exhausted",
            Self::DepthExceeded => "depth-exceeded",
            Self::SelectionFull => "selection-full",
        }
    }

    /// Whether this omission is a **gap** — something the caller asked for and did not get,
    /// as distinct from something it was never entitled to.
    ///
    /// `NotPermitted` is not a gap: the packet is complete with respect to what the caller
    /// may see. Everything else is.
    #[must_use]
    pub const fn is_gap(self) -> bool {
        !matches!(self, Self::NotPermitted)
    }
}

impl fmt::Display for Omission {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Stale { required, found } => {
                write!(
                    f,
                    "stale: required {} found {}",
                    required.value(),
                    found.value()
                )
            }
            other => f.write_str(other.name()),
        }
    }
}

/// The kind of a declared relationship between two sources.
///
/// T11 asks context records to *"distinguish required source-file coverage from task-critical
/// call/dependency/ownership relationship coverage"*. The set is closed: a relationship this
/// module cannot name is not one it can report as covered or missing.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum RelationKind {
    /// The source calls into the target.
    Call,
    /// The source depends on the target. Every edge declared through [`Assembly::register`]
    /// or [`Assembly::register_unreadable`] is of this kind.
    Dependency,
    /// The source owns, or is owned through, the target.
    Ownership,
}

impl RelationKind {
    /// Every kind, so a caller enumerating them cannot silently miss one added later.
    pub const ALL: [Self; 3] = [Self::Call, Self::Dependency, Self::Ownership];

    /// The stable wire name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Call => "call",
            Self::Dependency => "dependency",
            Self::Ownership => "ownership",
        }
    }
}

impl fmt::Display for RelationKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// One declared relationship of a selected source, and whether the packet covers its target.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Relation {
    /// The selected source that declared the relationship.
    pub from: String,
    /// The declared target.
    pub to: String,
    /// What kind of relationship it is.
    pub kind: RelationKind,
    /// `None` when the target is in the packet; otherwise why it is not.
    pub omission: Option<Omission>,
}

/// What a caller may draw from. Deny by default: a source absent from the permit is omitted
/// as [`Omission::NotPermitted`], never fetched.
#[derive(Clone, Debug, Default)]
pub struct Permit {
    allowed: Vec<String>,
}

impl Permit {
    /// An empty permit, which admits nothing.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Admit one source identity.
    ///
    /// # Errors
    ///
    /// [`Refusal::MalformedIdentity`] when `source` is not a lowercase hyphenated `UUIDv4`.
    pub fn allow(mut self, source: &str) -> Result<Self, Refusal> {
        let source = UuidV4::parse(source).map_err(Refusal::MalformedIdentity)?;
        let source = source.as_str().to_owned();
        if !self.allowed.contains(&source) {
            self.allowed.push(source);
        }
        Ok(self)
    }

    /// Whether this permit admits `source`.
    #[must_use]
    pub fn admits(&self, source: &str) -> bool {
        self.allowed.iter().any(|allowed| allowed == source)
    }

    /// How many sources are admitted.
    #[must_use]
    pub fn len(&self) -> usize {
        self.allowed.len()
    }

    /// Whether the permit admits nothing.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.allowed.is_empty()
    }
}

/// One registered source: its revision, its declared dependencies and its content, or the
/// fact that reading it failed.
#[derive(Clone, Debug)]
struct Source<'a> {
    identity: String,
    revision: Revision,
    dependencies: Vec<(String, RelationKind)>,
    content: Option<Content<'a>>,
}

/// One selected reference in a packet.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Selected<'a> {
    /// The source identity.
    pub identity: UuidV4<'a>,
    /// The revision that was included.
    pub revision: Revision,
    /// Its content.
    pub content: Content<'a>,
    /// How deep in the dependency chain it sat. The roots the caller named are depth 0.
    pub depth: u32,
}

/// An assembled context packet.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Packet<'a> {
    context: String,
    scope: Vec<String>,
    selected: Vec<Selected<'a>>,
    relations: Vec<Relation>,
    omissions: Vec<(String, Omission)>,
    bytes: u64,
    work: u64,
}

impl<'a> Packet<'a> {
    /// The caller-supplied stable context identity this packet was assembled for.
    #[must_use]
    pub fn context(&self) -> &str {
        &self.context
    }

    /// Every relationship declared by a selected source, in selection order and then in
    /// declaration order, each with its target's coverage.
    #[must_use]
    pub fn relations(&self) -> &[Relation] {
        &self.relations
    }

    /// The relationships whose target the caller needed and did not get.
    ///
    /// Kept apart from [`Packet::gaps`], which reports source coverage: a source can be in
    /// the packet while a call, dependency or ownership relationship it declares is not
    /// covered, and T11 requires the two to be distinguishable. A target omitted as
    /// [`Omission::NotPermitted`] is not a gap here either, for the same reason as there.
    #[must_use]
    pub fn relationship_gaps(&self) -> Vec<&Relation> {
        self.relations
            .iter()
            .filter(|relation| relation.omission.is_some_and(Omission::is_gap))
            .collect()
    }

    /// The selected references, in the packet's deterministic order.
    #[must_use]
    pub fn selected(&self) -> &[Selected<'a>] {
        &self.selected
    }

    /// Every source that is not in the packet, with its reason, sorted by identity so two
    /// assemblies of the same world produce byte-identical omission lists.
    #[must_use]
    pub fn omissions(&self) -> &[(String, Omission)] {
        &self.omissions
    }

    /// The omissions that are gaps — what the caller asked for and did not get.
    #[must_use]
    pub fn gaps(&self) -> Vec<(&str, Omission)> {
        self.omissions
            .iter()
            .filter(|(_, omission)| omission.is_gap())
            .map(|(identity, omission)| (identity.as_str(), *omission))
            .collect()
    }

    /// The total content bytes included.
    #[must_use]
    pub const fn bytes(&self) -> u64 {
        self.bytes
    }

    /// Selection and compaction work, in bytes examined.
    ///
    /// This is the figure `budget` accounts for. It counts bytes the assembly **looked at**,
    /// including those it then rejected for budget — work done is cost incurred, and
    /// charging only for what survived would make selection look free.
    #[must_use]
    pub const fn work(&self) -> u64 {
        self.work
    }

    /// The packet's cost as a [`budget::Usage`](crate::budget::Usage), ready to report.
    ///
    /// The provenance is [`Provenance::CheckerMeasured`] because the figure is measured by
    /// the engine while assembling, not settled by a worker.
    #[must_use]
    pub fn cost(&self) -> Usage {
        Usage::new(
            Amount::new(Unit::Bytes, self.work),
            Provenance::CheckerMeasured,
        )
    }
}

/// What changed between two packets, and which consumers are affected.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Change {
    /// Sources present in the new packet and not the old.
    pub added: Vec<String>,
    /// Sources present in the old packet and not the new.
    pub removed: Vec<String>,
    /// Sources in both, whose revision moved.
    pub revised: Vec<(String, Revision, Revision)>,
    /// The affected consumers: every selected source, in either packet, whose chain of one or
    /// more declared relationships reaches a changed source. Sorted and deduplicated.
    pub consumers: Vec<String>,
}

impl Change {
    /// Whether anything at all differs.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.removed.is_empty() && self.revised.is_empty()
    }

    /// Every changed source a consumer would have to re-read, sorted and deduplicated. The
    /// consumers themselves are [`Change::consumers`].
    #[must_use]
    pub fn affected(&self) -> Vec<&str> {
        let mut out: Vec<&str> = self
            .added
            .iter()
            .chain(&self.removed)
            .map(String::as_str)
            .chain(
                self.revised
                    .iter()
                    .map(|(identity, _, _)| identity.as_str()),
            )
            .collect();
        out.sort_unstable();
        out.dedup();
        out
    }
}

/// A registry of sources, from which packets are assembled.
///
/// There is no method that enumerates a filesystem, a vault or a directory. A source exists
/// here because a caller registered it, which is what makes *"do not load whole vaults by
/// default"* a property of the API rather than a habit.
#[derive(Clone, Debug, Default)]
pub struct Assembly<'a> {
    sources: Vec<Source<'a>>,
    /// Each registered identity's position in `sources`: the one lookup door.
    index: BTreeMap<String, usize>,
}

impl<'a> Assembly<'a> {
    /// An empty assembly.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The number of registered sources.
    #[must_use]
    pub fn len(&self) -> usize {
        self.sources.len()
    }

    /// Whether nothing is registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.sources.is_empty()
    }

    /// Register a readable source.
    ///
    /// # Errors
    ///
    /// * [`Refusal::MalformedIdentity`] for a source or dependency that is not a `UUIDv4`;
    /// * [`Refusal::DuplicateSource`] when the identity is already registered;
    /// * [`Refusal::RegistryFull`] when [`MAX_SOURCES`] sources are already registered, refused
    ///   before anything is copied;
    /// * [`Refusal::DependencyLimit`] beyond [`MAX_SELECTED`] declared dependencies;
    /// * [`Refusal::SourceTooLarge`] beyond [`MAX_SOURCE_BYTES`], refused at registration so
    ///   the bound is taken where the bytes are acquired rather than where they are copied.
    ///
    /// Every declared edge is a [`RelationKind::Dependency`]; [`Assembly::register_related`]
    /// declares typed ones.
    pub fn register(
        &mut self,
        identity: &str,
        revision: Revision,
        dependencies: &[&str],
        content: Content<'a>,
    ) -> Result<(), Refusal> {
        let related = as_dependencies(dependencies);
        self.register_related(identity, revision, &related, content)
    }

    /// Register a readable source whose declared relationships carry their kind.
    ///
    /// Every relationship is followed exactly as a dependency is; the kind is what lets a
    /// packet report [`Packet::relationship_gaps`] by call, dependency or ownership.
    ///
    /// # Errors
    ///
    /// As [`Assembly::register`], with [`Refusal::DependencyLimit`] beyond [`MAX_SELECTED`]
    /// declared relationships.
    pub fn register_related(
        &mut self,
        identity: &str,
        revision: Revision,
        relations: &[(&str, RelationKind)],
        content: Content<'a>,
    ) -> Result<(), Refusal> {
        if u64::try_from(content.len()).map_err(|_| Refusal::Overflow)? > MAX_SOURCE_BYTES {
            return Err(Refusal::SourceTooLarge);
        }
        self.insert(identity, revision, relations, Some(content))
    }

    /// Register a source whose read failed.
    ///
    /// It appears in packets as [`Omission::FetchFailed`], never as empty content: an empty
    /// success and a failed fetch mean different things to every consumer.
    ///
    /// # Errors
    ///
    /// As [`Assembly::register`], without the size check.
    pub fn register_unreadable(
        &mut self,
        identity: &str,
        revision: Revision,
        dependencies: &[&str],
    ) -> Result<(), Refusal> {
        let related = as_dependencies(dependencies);
        self.insert(identity, revision, &related, None)
    }

    fn insert(
        &mut self,
        identity: &str,
        revision: Revision,
        dependencies: &[(&str, RelationKind)],
        content: Option<Content<'a>>,
    ) -> Result<(), Refusal> {
        if dependencies.len() > MAX_SELECTED {
            return Err(Refusal::DependencyLimit);
        }
        let identity = UuidV4::parse(identity).map_err(Refusal::MalformedIdentity)?;
        if self.find(identity.as_str()).is_some() {
            return Err(Refusal::DuplicateSource);
        }
        if self.sources.len() >= MAX_SOURCES {
            return Err(Refusal::RegistryFull);
        }
        let mut declared = Vec::with_capacity(dependencies.len());
        for (dependency, kind) in dependencies {
            let dependency = UuidV4::parse(dependency).map_err(Refusal::MalformedIdentity)?;
            declared.push((dependency.as_str().to_owned(), *kind));
        }
        self.index
            .insert(identity.as_str().to_owned(), self.sources.len());
        self.sources.push(Source {
            identity: identity.as_str().to_owned(),
            revision,
            dependencies: declared,
            content,
        });
        Ok(())
    }

    fn find(&self, identity: &str) -> Option<usize> {
        self.index.get(identity).copied()
    }
}

impl<'a> Assembly<'a> {
    /// Assemble a packet from `roots`, following declared dependencies breadth-first.
    ///
    /// Ordering is deterministic and does not depend on registration order or on any hash
    /// iteration: roots are visited in the order the caller named them, and each source's
    /// dependencies in the order it declared them. Two assemblies of the same world produce
    /// the same packet, which is the contract's *"reproducible packet ordering"*.
    ///
    /// Breadth-first matters: it means a shallow source is never displaced from the budget
    /// by a deep one, so exhausting the budget truncates the *edge* of the graph rather than
    /// an arbitrary slice of it.
    ///
    /// `context_id` is the caller's stable context identity. The packet carries it and the
    /// permit's scope, so [`Assembly::compare`] can refuse to treat two different contexts as
    /// one refreshed context.
    ///
    /// # Errors
    ///
    /// * [`Refusal::IncompatibleUnit`] when `budget` is not in [`Unit::Bytes`]. A token
    ///   budget is refused too: no token estimator is admitted, so it has no byte meaning;
    /// * [`Refusal::BudgetTooLarge`] beyond [`MAX_PACKET_BYTES`], taken before any content
    ///   is copied;
    /// * [`Refusal::MalformedIdentity`] when `context_id` is not a `UUIDv4`;
    /// * [`Refusal::RootLimit`] for more than [`MAX_ROOTS`] roots, before any root is read;
    /// * [`Refusal::MalformedIdentity`] for a root that is not a `UUIDv4`;
    /// * [`Refusal::Overflow`] on a byte sum that leaves `u64`.
    pub fn assemble(
        &'a self,
        context_id: &str,
        roots: &[&str],
        required: Revision,
        permit: &Permit,
        budget: Amount,
    ) -> Result<Packet<'a>, Refusal> {
        if budget.unit() != Unit::Bytes {
            return Err(Refusal::IncompatibleUnit);
        }
        if budget.value() > MAX_PACKET_BYTES {
            return Err(Refusal::BudgetTooLarge);
        }
        let context_id = UuidV4::parse(context_id).map_err(Refusal::MalformedIdentity)?;
        // The root list is caller input the queue copies whole, so its bound is taken here,
        // before any root is parsed or queued.
        if roots.len() > MAX_ROOTS {
            return Err(Refusal::RootLimit);
        }
        let mut queue: Vec<(String, u32)> = Vec::new();
        for root in roots {
            let root = UuidV4::parse(root).map_err(Refusal::MalformedIdentity)?;
            queue.push((root.as_str().to_owned(), 0));
        }
        let mut seen: Vec<String> = Vec::new();
        let mut omissions: BTreeMap<String, Omission> = BTreeMap::new();
        let mut selected: Vec<Selected<'a>> = Vec::new();
        let mut selected_sources: Vec<&Source<'a>> = Vec::new();
        let mut bytes: u64 = 0;
        let mut work: u64 = 0;
        let mut head = 0;
        // The walk takes its own budget. `seen` stops a source being processed twice, but
        // that is a property of the data; this is a property of the loop, and the two fail
        // differently. Without it a mutation of the cursor arithmetic does not fail the
        // traversal, it hangs it — which reads as a tooling problem rather than a defect
        // (mutation testing reports TIMEOUT, not a killed mutant). The bound is the largest
        // queue this walk can legitimately build: one entry per root plus one per declared
        // dependency of every selectable source.
        let mut steps = StepBudget::for_roots(roots.len());

        while head < queue.len() {
            steps.spend()?;
            let (identity, depth) = queue[head].clone();
            head += 1;
            if seen.contains(&identity) {
                continue;
            }
            seen.push(identity.clone());

            if depth > MAX_DEPTH {
                omissions.insert(identity, Omission::DepthExceeded);
                continue;
            }
            if !permit.admits(&identity) {
                omissions.insert(identity, Omission::NotPermitted);
                continue;
            }
            let Some(index) = self.find(&identity) else {
                omissions.insert(identity, Omission::Missing);
                continue;
            };
            let source = &self.sources[index];
            if source.revision < required {
                omissions.insert(
                    identity,
                    Omission::Stale {
                        required,
                        found: source.revision,
                    },
                );
                continue;
            }
            let Some(content) = source.content else {
                omissions.insert(identity, Omission::FetchFailed);
                continue;
            };
            // Work is charged for every byte examined, including bytes then rejected for
            // budget: selection is not free just because its result was discarded.
            let size = u64::try_from(content.len()).map_err(|_| Refusal::Overflow)?;
            work = work.checked_add(size).ok_or(Refusal::Overflow)?;
            if selected.len() >= MAX_SELECTED {
                omissions.insert(identity, Omission::SelectionFull);
                continue;
            }
            let projected = bytes.checked_add(size).ok_or(Refusal::Overflow)?;
            if projected > budget.value() {
                omissions.insert(identity, Omission::BudgetExhausted);
                continue;
            }
            bytes = projected;
            selected.push(Selected {
                identity: UuidV4::parse(source.identity.as_str())
                    .map_err(Refusal::MalformedIdentity)?,
                revision: source.revision,
                content,
                depth,
            });
            selected_sources.push(source);
            for (dependency, _) in &source.dependencies {
                queue.push((dependency.clone(), depth + 1));
            }
        }

        let relations = relations_of(&selected_sources, &omissions);
        let mut scope = permit.allowed.clone();
        scope.sort_unstable();
        Ok(Packet {
            context: context_id.as_str().to_owned(),
            scope,
            relations,
            selected,
            omissions: omissions.into_iter().collect(),
            bytes,
            work,
        })
    }

    /// Compare two packets and name what a consumer must re-read.
    ///
    /// Both lists are sorted, so the report is reproducible regardless of the order the
    /// packets were assembled in.
    ///
    /// # Errors
    ///
    /// [`Refusal::ContextMismatch`] when the two packets were assembled for different context
    /// identities or under different permitted scopes: their difference would not be a
    /// refresh of one context, and reporting it as one would name the wrong consumers.
    pub fn compare(old: &Packet<'_>, new: &Packet<'_>) -> Result<Change, Refusal> {
        if old.context != new.context || old.scope != new.scope {
            return Err(Refusal::ContextMismatch);
        }
        let index = |packet: &Packet<'_>| -> BTreeMap<String, Revision> {
            packet
                .selected()
                .iter()
                .map(|item| (item.identity.as_str().to_owned(), item.revision))
                .collect()
        };
        let (before, after) = (index(old), index(new));
        let added = after
            .keys()
            .filter(|identity| !before.contains_key(*identity))
            .cloned()
            .collect();
        let removed = before
            .keys()
            .filter(|identity| !after.contains_key(*identity))
            .cloned()
            .collect();
        let revised = before
            .iter()
            .filter_map(|(identity, old_revision)| {
                after.get(identity).and_then(|new_revision| {
                    (old_revision != new_revision)
                        .then(|| (identity.clone(), *old_revision, *new_revision))
                })
            })
            .collect();
        let mut change = Change {
            added,
            removed,
            revised,
            consumers: Vec::new(),
        };
        change.consumers = consumers(&change.affected(), old, new);
        Ok(change)
    }
}

/// Untyped declarations as [`RelationKind::Dependency`] relationships.
///
/// At most one more than [`MAX_SELECTED`] is copied: that is enough for the registration to
/// refuse by [`Refusal::DependencyLimit`], and no more is acquired from an over-long list.
fn as_dependencies<'d>(dependencies: &[&'d str]) -> Vec<(&'d str, RelationKind)> {
    dependencies
        .iter()
        .take(MAX_SELECTED.saturating_add(1))
        .map(|dependency| (*dependency, RelationKind::Dependency))
        .collect()
}

/// Every relationship a selected source declared, with its target's coverage, in selection
/// order and then declaration order.
///
/// The set is derived, so it takes its bound from the two it is derived from: at most
/// [`MAX_SELECTED`] sources, each declaring at most [`MAX_SELECTED`] relationships.
fn relations_of(selected: &[&Source<'_>], omissions: &BTreeMap<String, Omission>) -> Vec<Relation> {
    let mut relations = Vec::new();
    for source in selected {
        for (target, kind) in &source.dependencies {
            relations.push(Relation {
                from: source.identity.clone(),
                to: target.clone(),
                kind: *kind,
                omission: omissions.get(target).copied(),
            });
        }
    }
    relations
}

/// Every selected source, in either packet, whose chain of one or more declared relationships
/// reaches one of `changed`, sorted.
///
/// The walk goes backwards over the relationships both packets recorded, so it needs no second
/// registry: a consumer is exactly a declared relationship read from its target's side.
fn consumers(changed: &[&str], old: &Packet<'_>, new: &Packet<'_>) -> Vec<String> {
    let mut declared_by: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for relation in old.relations.iter().chain(&new.relations) {
        declared_by
            .entry(relation.to.as_str())
            .or_default()
            .push(relation.from.as_str());
    }
    let mut frontier: Vec<&str> = changed.to_vec();
    let mut found: BTreeSet<&str> = BTreeSet::new();
    while let Some(target) = frontier.pop() {
        for consumer in declared_by.get(target).into_iter().flatten() {
            if found.insert(consumer) {
                frontier.push(consumer);
            }
        }
    }
    found.into_iter().map(str::to_owned).collect()
}

#[cfg(test)]
mod step_budget_tests {
    use super::{MAX_SELECTED, Refusal, StepBudget};

    /// The boundary, stated from the other side: the limit is how many steps are ALLOWED, so
    /// the `limit`-th spend succeeds and the `limit + 1`-th refuses. `>=` against `>` and
    /// `==` differ exactly here, and all three survived while this decision was inline in
    /// the walk, where no input could reach it.
    #[test]
    fn the_last_allowed_step_succeeds_and_the_next_refuses() {
        let mut budget = StepBudget::for_roots(0);
        let limit = budget.limit();
        for step in 1..=limit {
            assert_eq!(
                budget.spend(),
                Ok(()),
                "step {step} of {limit} should be allowed"
            );
        }
        assert_eq!(budget.spent(), limit);
        assert_eq!(budget.spend(), Err(Refusal::TraversalBudget));
        // A refused spend costs nothing: a budget that kept counting past its limit would
        // report a spend total no walk could have produced.
        assert_eq!(budget.spent(), limit);
        assert_eq!(budget.spend(), Err(Refusal::TraversalBudget));
    }

    #[test]
    fn a_fresh_budget_has_spent_nothing() {
        let budget = StepBudget::for_roots(3);
        assert_eq!(budget.spent(), 0);
    }

    #[test]
    fn each_spend_advances_by_exactly_one() {
        // Pinned off the origin: three spends, each asserted, so a budget frozen at 1 or
        // one that doubled would both fail. A single spend expecting 1 pins neither.
        let mut budget = StepBudget::for_roots(0);
        for expected in 1..=3_usize {
            assert_eq!(budget.spend(), Ok(()));
            assert_eq!(budget.spent(), expected);
        }
    }

    #[test]
    fn the_limit_is_one_queue_entry_per_root_plus_the_selectable_dependency_product() {
        // The two numbers, computed here rather than read from the constructor, so a change
        // to either the shape or the constant is a red test and not a silent widening.
        for roots in [0_usize, 1, 7, MAX_SELECTED] {
            assert_eq!(
                StepBudget::for_roots(roots).limit(),
                MAX_SELECTED * (MAX_SELECTED + 1) + roots,
                "budget for {roots} roots"
            );
        }
    }

    #[test]
    fn more_roots_buy_more_steps() {
        assert!(StepBudget::for_roots(2).limit() > StepBudget::for_roots(1).limit());
        assert_eq!(
            StepBudget::for_roots(2).limit() - StepBudget::for_roots(1).limit(),
            1
        );
    }

    #[test]
    fn an_absurd_root_count_saturates_rather_than_wrapping() {
        // `for_roots` is `const` and saturating; a wrapping add here would hand the walk a
        // tiny budget and turn a bound into a refusal on the first step.
        let budget = StepBudget::for_roots(usize::MAX);
        assert_eq!(budget.limit(), usize::MAX);
        let mut budget = budget;
        assert_eq!(budget.spend(), Ok(()));
    }
}
