# HermesClaw Architecture Summary

**Authority:** canonical architecture summary  
**Detailed reference:** `90_ARCHITECTURE_BASELINE_FULL.md`

## Product shape

```text
React / TypeScript UI      CLI / Remote API / Channels
          |                         |
          +------------+------------+
                       |
                 HermesClaw API
                       |
                 Mission Engine
                       |
       +---------------+---------------+
       |               |               |
   Agent Engine   Capability Engine  Memory Engine
       |               |               |
       +----------+----+------+---------+
                  |           |
             Model Fabric  Policy Kernel
                  |           |
        +---------+----+------+---------+
        |              |                |
     Gateway       Automation         Nodes
        |              |                |
        +--------------+----------------+
                       |
                 Evidence Engine
                       |
              Canonical State/Audit
```

## Core rules

- Internal messages use stable typed contracts.
- Model output is data, never authorization.
- Every external input carries provenance/trust metadata.
- All externally consequential actions go through policy.
- Durable concerns have one canonical owner.
- Compatibility boundaries cannot become shadow databases.
- The same Rust core should support desktop, headless/server, CLI, and constrained node modes.

## Target crate domains

Long-term crate boundaries include domain, agent, mission, models, tools, policy, memory, skills, automation, gateway, nodes, browser, computer, process, state, events, evidence, observability, API, and temporary compatibility adapters.

Do not create all crates merely because they appear in the architecture. Add a domain only when the next vertical capability requires it.

## Mission lifecycle

Canonical conceptual states:

`created -> planning -> executing -> waiting_approval | waiting_external | verifying -> completed | failed | cancelled`

## Tool execution

`model request -> schema validation -> canonicalization -> risk classification -> policy -> optional human approval -> isolated execution -> output sanitation -> evidence -> typed result`
