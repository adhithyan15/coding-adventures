# COS01: Personal Chief of Staff — Exploration Spec

**Status:** Draft / Exploration
**Date:** 2026-03-22
**Author:** Claude + Adhithya

---

## 1. Vision

A personal "Chief of Staff" (CoS) — an AI agent that manages the day-to-day logistics of your life and your family's life. Think of what a real chief of staff does for a CEO: they triage incoming requests, keep the calendar sane, make sure nothing falls through the cracks, surface what matters, and handle the rest quietly.

The key insight: this isn't a chatbot you query. It's an **agent that runs continuously**, maintains state about your life, and proactively reaches out to you when something needs attention.

---

## 2. What a Chief of Staff Actually Does

A real chief of staff handles these categories of work:

### 2.1 Triage & Filtering
- **Email/message triage** — reads incoming email, classifies by urgency, drafts responses for routine items, escalates the rest
- **Notification management** — aggregates notifications across apps, filters noise, surfaces signal
- **Decision queuing** — "these 5 things need your decision today, here's the context for each"

### 2.2 Calendar & Scheduling
- **Calendar management** — resolves conflicts, blocks focus time, manages meeting prep
- **Family calendar coordination** — kids' school events, doctor appointments, activities, playdates
- **Travel logistics** — booking, itinerary management, document checklists

### 2.3 Family Logistics
- **Grocery/meal planning** — weekly meal plan, shopping list, dietary considerations
- **Household maintenance** — track when things need servicing, coordinate with providers
- **Kids' school** — homework deadlines, permission slips, school communications
- **Medical** — appointment scheduling, prescription refills, insurance claims

### 2.4 Financial Oversight
- **Bill tracking** — due dates, autopay verification, anomaly detection
- **Budget monitoring** — spending vs. budget by category, alerts on unusual charges
- **Subscription management** — what you pay for, what you actually use, cancellation reminders

### 2.5 Information & Research
- **"Hey, can you find out..."** — research tasks with a deadline
- **Daily briefing** — weather, calendar summary, reminders, news relevant to your interests
- **Gift/event planning** — birthdays, anniversaries, with gift ideas and reminders

---

## 3. Architecture Options

### 3.1 Option A: Minimal Agent (Recommended Starting Point)

```
┌──────────────────────────────────────┐
│           Chief of Staff Agent       │
│                                      │
│  ┌──────────┐  ┌──────────────────┐  │
│  │ Scheduler│  │  Skill Registry  │  │
│  │ (cron)   │  │  (pluggable)     │  │
│  └──────────┘  └──────────────────┘  │
│                                      │
│  ┌──────────┐  ┌──────────────────┐  │
│  │ State DB │  │  LLM Interface   │  │
│  │ (SQLite) │  │  (Claude API)    │  │
│  └──────────┘  └──────────────────┘  │
│                                      │
│  ┌──────────────────────────────────┐│
│  │     Integration Layer            ││
│  │  Google Calendar · Gmail · etc.  ││
│  └──────────────────────────────────┘│
└──────────────────────────────────────┘
         │               │
    ┌────┴────┐    ┌─────┴─────┐
    │Telegram │    │  Daily    │
    │  Bot    │    │  Email    │
    └─────────┘    │  Digest   │
                   └───────────┘
```

**Core components:**
- **Scheduler** — runs periodic tasks (check email, scan calendar, generate daily briefing)
- **Skill Registry** — pluggable modules for each domain (calendar, email, groceries, etc.)
- **State DB** — SQLite for persistence (family members, preferences, task history, context)
- **LLM Interface** — Claude API for reasoning, summarization, drafting
- **Integration Layer** — OAuth connections to external services
- **Notification Channel** — Telegram bot or email digest for communication

**Why start here:** Small, self-hosted, no framework dependency. You control every line. Add skills incrementally as you figure out what's actually useful vs. what sounds cool.

### 3.2 Option B: Agent Framework (Claude Agent SDK, LangGraph, etc.)

Use an existing agent framework to handle the orchestration plumbing. You focus on defining skills and integrations.

**Pros:** Less boilerplate, built-in patterns for tool use, memory, and multi-step reasoning.
**Cons:** Framework lock-in, harder to debug, may be overkill for v1.

### 3.3 Option C: OpenClaw-Based

Use OpenClaw's architecture (computer use, skill system, etc.) as the foundation.

**Pros:** Ambitious capabilities out of the box (can interact with any web UI).
**Cons:** Security concerns (runs with broad system access), complex to lock down for family data, opinionated architecture you may not need.

---

## 4. Security Model (Critical for Family Data)

This system will handle deeply personal data. Security isn't a feature — it's a precondition.

### 4.1 Principles
1. **Minimal access** — each skill gets only the API scopes it needs
2. **Local-first storage** — family data stays on hardware you control
3. **Encrypted at rest** — SQLite with SQLCipher or similar
4. **Audit log** — every action the agent takes is logged and reviewable
5. **Human-in-the-loop for high-stakes** — spending money, sending messages, deleting things → always confirm
6. **No training on your data** — use Claude API with data retention disabled

### 4.2 Threat Model
| Threat | Mitigation |
|--------|-----------|
| API key leak | Vault/keyring storage, rotate regularly |
| LLM prompt injection via email | Sanitize all external content before including in prompts |
| Runaway agent (sends wrong email, books wrong thing) | Action confirmation for anything external |
| Data breach on host machine | Full-disk encryption, SQLCipher, firewall |
| Cloud provider reads your data | Self-host, or use E2E encrypted sync |

---

## 5. Suggested Build Order

Start narrow, prove value, then expand. Each phase should be independently useful.

### Phase 0: Foundation (Week 1)
- [ ] Core agent loop (scheduler + skill dispatch)
- [ ] SQLite database with encrypted storage
- [ ] Claude API integration with system prompt
- [ ] Telegram bot for bidirectional communication
- [ ] Basic "ask me anything" skill (you message it, it responds)

### Phase 1: Daily Briefing (Week 2)
- [ ] Google Calendar integration (read-only)
- [ ] Weather API integration
- [ ] Morning briefing skill: "Here's your day — 3 meetings, rain at 2pm, kid's soccer at 4"
- [ ] Evening summary: "Here's what happened today, here's what's tomorrow"

### Phase 2: Calendar Management (Week 3-4)
- [ ] Calendar write access (create/modify events)
- [ ] Conflict detection and resolution suggestions
- [ ] Family member calendar aggregation
- [ ] "Schedule a dentist appointment for the kids next week" → finds slots, proposes times

### Phase 3: Email Triage (Week 4-5)
- [ ] Gmail API integration (read)
- [ ] Email classification (urgent / needs response / FYI / spam)
- [ ] Daily email digest: "12 new emails, 2 need your attention"
- [ ] Draft responses for routine emails (with your approval)

### Phase 4: Family Logistics (Week 6+)
- [ ] Grocery list management
- [ ] Meal planning with dietary preferences
- [ ] Household task tracking
- [ ] Kids' school calendar sync

### Phase 5: Financial (Week 8+)
- [ ] Bank/credit card read access (Plaid or similar)
- [ ] Bill due date tracking
- [ ] Spending anomaly alerts
- [ ] Subscription audit

---

## 6. Technology Choices (To Decide)

| Decision | Options | Leaning |
|----------|---------|---------|
| Language | Python, TypeScript, Go | Python (fastest to prototype, best LLM library ecosystem) |
| Database | SQLite + SQLCipher, Postgres | SQLite (simple, local, portable) |
| LLM | Claude API, local models | Claude API (quality matters for family context) |
| Chat interface | Telegram, Discord, Signal, WhatsApp, custom | Telegram (best bot API, E2E optional) |
| Hosting | Home server, cloud VPS, both | Start local, add cloud relay later |
| Calendar | Google Calendar, Apple Calendar | Google (better API) |
| Email | Gmail API, IMAP | Gmail API (structured, OAuth) |
| Auth/Secrets | OS keyring, Vault, .env | OS keyring for local, Vault for cloud |

---

## 7. What Makes This Different from Existing Tools

- **Siri/Alexa/Google Assistant** — reactive, no persistent context, can't reason across domains
- **Notion/Todoist** — manual input, no proactive behavior
- **OpenClaw/Open Interpreter** — general-purpose, not tailored to family life, security concerns
- **Lindy.ai / Zapier AI** — cloud-hosted, your data goes to their servers

The differentiation is: **a self-hosted, security-first agent that understands your family's context and proactively manages logistics** rather than waiting to be asked.

---

## 8. Open Questions

1. **Multi-user access** — Does your spouse/partner also interact with it? Shared context or separate?
2. **Kids' access** — Can kids ask it questions? ("What's for dinner?" / "When is soccer?")
3. **Voice interface** — Is voice important, or is chat sufficient?
4. **Mobile access** — Phone notifications essential, or desktop-first is fine?
5. **Offline capability** — Does it need to work without internet (local LLM fallback)?
6. **Budget** — What's the acceptable monthly cost for API calls + hosting?
7. **Privacy boundaries** — Any categories explicitly off-limits? (e.g., "don't read my personal texts")
8. **Existing tools** — What calendar/email/todo systems does your family currently use?

---

## 9. Next Steps

Once you've had a chance to review this and answer the open questions, we can:

1. **Pick a phase** — probably Phase 0 + 1 together (foundation + daily briefing)
2. **Write a detailed technical spec** for that phase
3. **Build it** — right here in this repo under `code/programs/python/chief-of-staff/`

This is the kind of project that's best built incrementally. A working daily briefing in week 1 is better than a perfect architecture diagram that takes a month.
