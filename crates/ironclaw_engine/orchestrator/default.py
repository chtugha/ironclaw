# Engine v2 Orchestrator (default, v0)
#
# This is the self-modifiable execution loop. It replaces the Rust
# ExecutionLoop::run() with Python that can be patched at runtime
# by the self-improvement Mission.
#
# Host functions (provided by Rust via Monty suspension):
#   __llm_complete__(messages, actions, config)  -> response dict
#   __execute_code_step__(code, state)           -> result dict
#   __execute_action__(name, params)             -> result dict
#   __execute_actions_parallel__(calls)          -> list of result dicts (parallel execution)
#   __check_signals__()                          -> None | "stop" | {"inject": msg}
#   __emit_event__(kind, **data)                 -> None
#   __save_checkpoint__(state, counters)         -> None
#   __transition_to__(state, reason)             -> None
#   __retrieve_docs__(goal, max_docs)            -> list of doc dicts
#   __check_budget__()                           -> budget dict
#   __get_actions__()                            -> list of action dicts
#   __list_skills__()                            -> list of skill dicts
#   __record_skill_usage__(doc_id, success)      -> None
#   __regex_match__(pattern, text)               -> bool
#   __set_active_skills__(skills)                -> None
#   __apply_token_guard__(parts)                 -> {"dropped": [...], "fits": bool, "survivors": {...}, "system_prompt": str, "conversation_history": [...]}
#   __save_plan_doc__(goal, steps, is_decomposition) -> doc_id str | None
#
# Context variables (injected by Rust before execution):
#   context  - list of prior messages [{role, content}]
#   goal     - thread goal string
#   actions  - list of available action defs
#   state    - persisted state dict from prior steps
#   config   - thread config dict


import re


def _token_count(text):
    if not text:
        return 0
    return max(1, int(len(text.encode("utf-8")) * 0.25))


def _do_transition(target, reason, config=None):
    if config and config.get("_suppress_transitions"):
        return
    __transition_to__(target, reason)


# ── Helper functions (self-modifiable glue) ──────────────────
# Defined before run_loop so they are in scope when called.


def extract_final(text):
    """Extract FINAL() content from text. Returns None if not found."""
    idx = text.find("FINAL(")
    if idx < 0:
        return None
    after = text[idx + 6:]
    # Handle triple-quoted strings
    for q in ['"""', "'''"]:
        if after.startswith(q):
            end = after.find(q, len(q))
            if end >= 0:
                return after[len(q):end]
    # Handle single/double quoted strings
    if after and after[0] in ('"', "'"):
        quote = after[0]
        end = after.find(quote, 1)
        if end >= 0:
            return after[1:end]
    # Handle balanced parens
    depth = 1
    for i, ch in enumerate(after):
        if ch == "(":
            depth += 1
        elif ch == ")":
            depth -= 1
            if depth == 0:
                return after[:i]
    return None


def strip_quoted_strings(line):
    """Remove double-quoted string literals from a line."""
    result = []
    in_quote = False
    escaped = False
    for ch in line:
        if escaped:
            escaped = False
            if not in_quote:
                result.append(ch)
            continue
        if ch == "\\":
            escaped = True
            if not in_quote:
                result.append(ch)
            continue
        if ch == '"':
            in_quote = not in_quote
            continue
        if not in_quote:
            result.append(ch)
    return "".join(result)


def strip_code_blocks(text):
    """Strip fenced code blocks, indented code lines, and double-quoted strings."""
    result = []
    in_fence = False
    for line in text.split("\n"):
        trimmed = line.lstrip()
        if trimmed.startswith("```"):
            in_fence = not in_fence
            continue
        if in_fence:
            continue
        if line.startswith("    ") or line.startswith("\t"):
            continue
        result.append(strip_quoted_strings(line))
    return "\n".join(result)


def signals_tool_intent(text):
    """Detect when text expresses intent to call a tool without actually doing so.

    Ported from V1 Rust llm_signals_tool_intent(): strips code blocks and
    quoted strings, checks exclusion phrases, then requires a future-tense
    prefix ("let me", "I'll", "I will", "I'm going to") immediately followed
    by an action verb ("search", "fetch", "check", etc.).
    """
    stripped = strip_code_blocks(text)
    lower = stripped.lower()

    EXCLUSIONS = [
        "let me explain", "let me know", "let me think",
        "let me summarize", "let me clarify", "let me describe",
        "let me help", "let me understand", "let me break",
        "let me outline", "let me walk you", "let me provide",
        "let me suggest", "let me elaborate", "let me start by",
    ]
    for exc in EXCLUSIONS:
        if exc in lower:
            return False

    PREFIXES = ["let me ", "i'll ", "i will ", "i'm going to "]
    ACTION_VERBS = [
        "search", "look up", "check", "fetch", "find",
        "read the", "write the", "create", "run the", "execute",
        "query", "retrieve", "add it", "add the", "add this",
        "add that", "update the", "delete", "remove the", "look into",
        "stop", "pause", "cancel", "halt", "disable",
    ]

    for prefix in PREFIXES:
        start = 0
        while True:
            i = lower.find(prefix, start)
            if i < 0:
                break
            after = lower[i + len(prefix):]
            for verb in ACTION_VERBS:
                if after.startswith(verb) or (" " + verb) in after.split("\n")[0]:
                    return True
            start = i + 1

    return False


def signals_execution_intent(text):
    """Detect explicit execution commands in user messages.

    Ported from Rust user_signals_execution_intent(): strips code blocks and
    quoted strings, then checks for imperative verb phrases that require action.
    Deliberately excludes context-dependent phrases ("go ahead", "yes do it")
    that require multi-turn understanding.
    """
    stripped = strip_code_blocks(text)
    lower = stripped.lower()

    EXEC_PHRASES = [
        "run it", "run that", "run them", "run this", "run the ",
        "execute it", "execute that", "execute them", "execute this",
        "execute the ",
        "ship it", "deploy it", "deploy that", "deploy this", "deploy the ",
        "send it", "send that", "send the ",
        "fetch it", "fetch that", "fetch the ",
        "stop it", "stop that", "stop this", "stop the ",
        "pause it", "pause that", "pause this", "pause the ",
        "cancel it", "cancel that", "cancel this", "cancel the ",
        "halt it", "halt that", "halt this", "halt the ",
        "disable it", "disable that", "disable this", "disable the ",
        "please run ", "please execute ", "please fetch ",
        "please send ", "please deploy ",
        "please stop ", "please pause ", "please cancel ",
        "please halt ", "please disable ",
    ]
    if any(phrase in lower for phrase in EXEC_PHRASES):
        return True

    # Bare imperative commands at the start of the message.
    # "stop pinging", "stop", "pause", "cancel" are unambiguous commands
    # that don't match the "verb + pronoun/article" pattern above.
    # Checking startswith avoids false positives like "I can't stop".
    # Strip trailing punctuation so "Stop." and "cancel!" still match.
    trimmed = lower.strip().rstrip(".,!?;:")
    IMPERATIVE_STARTS = ["stop ", "pause ", "cancel ", "halt ", "disable "]
    BARE_COMMANDS = ["stop", "pause", "cancel", "halt", "disable"]
    if trimmed in BARE_COMMANDS:
        return True
    if any(trimmed.startswith(s) for s in IMPERATIVE_STARTS):
        return True

    return False


def format_output(result, max_chars=8000):
    """Format code execution result for the next LLM context message."""
    parts = []

    stdout = result.get("stdout", "")
    if stdout:
        parts.append("[stdout]\n" + stdout)

    for r in result.get("action_results", []):
        name = r.get("action_name", "?")
        output = str(r.get("output", ""))
        if r.get("is_error"):
            parts.append("[" + name + " ERROR] " + output)
        else:
            if len(output) > 500:
                preview = output[:500] + "..."
                parts.append(
                    "[" + name + "] " + preview +
                    "\n(full result stored in state['" + name + "']; "
                    "do NOT retype the data — reference the variable in your next call.)"
                )
            else:
                parts.append("[" + name + "] " + output)

    ret = result.get("return_value")
    if ret is not None:
        parts.append("[return] " + str(ret))

    text = "\n\n".join(parts)

    # Truncate from the front (keep the tail with most recent results)
    if len(text) > max_chars:
        text = "... (truncated) ...\n" + text[-max_chars:]

    if not text:
        text = "[code executed, no output]"

    return text


def format_docs(docs):
    """Format memory docs for context injection."""
    parts = ["## Prior Knowledge (from completed threads)\n"]
    for doc in docs:
        label = doc.get("type", "NOTE").upper()
        content = doc.get("content", "")[:500]
        truncated = "..." if len(doc.get("content", "")) > 500 else ""
        parts.append("### [" + label + "] " + doc.get("title", "") +
                      "\n" + content + truncated + "\n")
    return "\n".join(parts)


def ensure_working_messages(state, context):
    """Initialize the mutable orchestrator transcript."""
    existing = state.get("working_messages")
    if isinstance(existing, list):
        return existing
    if isinstance(context, list):
        state["working_messages"] = list(context)
    else:
        state["working_messages"] = []
    return state["working_messages"]


def append_message(messages, role, content, action_name=None, action_call_id=None, action_calls=None):
    """Append a normalized message to the working transcript."""
    msg = {"role": role, "content": content}
    if action_name is not None:
        msg["action_name"] = action_name
    if action_call_id is not None:
        msg["action_call_id"] = action_call_id
    if action_calls is not None:
        msg["action_calls"] = action_calls
    messages.append(msg)


def append_system_append(messages, content):
    """Append additional context to the first system message."""
    for msg in messages:
        if msg.get("role") == "System":
            existing = msg.get("content", "")
            if existing:
                msg["content"] = existing + "\n\n" + content
            else:
                msg["content"] = content
            return
    messages.insert(0, {"role": "System", "content": content})


# Conservative fallback heuristic matching the old Rust-side estimator.
# These MUST be defined before `estimate_context_tokens` (and therefore
# before the `FINAL(result)` entry-point call below). Moving them after the
# entry point is a latent NameError every time `compact_if_needed` runs.
CHARS_PER_TOKEN = 4
MESSAGE_OVERHEAD_CHARS = 4


def estimate_context_tokens(messages):
    """Estimate token count for a transcript using a rough chars/token heuristic."""
    total_chars = 0
    for msg in messages:
        total_chars += len(msg.get("content", ""))
        total_chars += len(msg.get("action_name", "") or "")
        total_chars += MESSAGE_OVERHEAD_CHARS
    return (total_chars + CHARS_PER_TOKEN - 1) // CHARS_PER_TOKEN


def compact_if_needed(state, config):
    """Compact thread context when the active message history grows too large.

    The orchestrator owns compaction policy. Rust only provides helpers for
    token estimation, explicit LLM calls, and replacing the active message
    scaffold after a summary has been produced.
    """
    if not config.get("enable_compaction", False):
        return False

    context_limit = config.get("model_context_limit", 128000)
    threshold_pct = config.get("compaction_threshold", 0.85)
    threshold = int(context_limit * threshold_pct)
    working_messages = state.get("working_messages")
    if not isinstance(working_messages, list) or not working_messages:
        return False

    current_tokens = estimate_context_tokens(working_messages)
    if current_tokens < threshold:
        return False

    snapshot = list(working_messages)

    history = state.get("history")
    if not isinstance(history, list):
        history = []
        state["history"] = history

    compaction_count = state.get("compaction_count", 0) + 1
    history.append({
        "kind": "compaction",
        "index": compaction_count,
        "tokens_before": current_tokens,
        "message_count": len(snapshot),
    })
    if len(history) > 3:
        history[:] = history[-3:]

    summary_prompt = (
        "Summarize progress so far in a concise but complete way.\n"
        "Include:\n"
        "1. What has been accomplished\n"
        "2. Key intermediate results, facts, and variable values\n"
        "3. Tool results or findings worth preserving\n"
        "4. What still needs to be done\n"
        "5. Errors encountered and how they were handled\n\n"
        "Preserve all information needed to continue the task."
    )
    summary_messages = list(snapshot)
    summary_messages.append({"role": "User", "content": summary_prompt})
    summary_resp = __llm_complete__(summary_messages, None, {"force_text": True})

    summary_text = summary_resp.get("content", "")
    if not summary_text:
        summary_text = "[compaction produced no summary]"

    state["working_messages"] = []
    system_message = None
    for msg in snapshot:
        if msg.get("role") == "System":
            system_message = {"role": "System", "content": msg.get("content", "")}
            break
    if system_message is not None:
        state["working_messages"].append(system_message)
    append_message(state["working_messages"], "Assistant", summary_text)
    append_message(
        state["working_messages"],
        "User",
        "Your conversation has been compacted. The summary above captures prior progress. "
        "Older details remain available through state['history'] and project retrieval. Continue working on the task.",
    )
    state["compaction_count"] = compaction_count
    return True


# ── Skill selection and injection (self-modifiable) ────────


# Smart-quote / smart-dash characters that auto-correct produces on iOS,
# macOS, and most rich text inputs. Skill activation patterns and keywords
# are authored with ASCII punctuation, so a typed `I'm a CEO` (curly
# apostrophe U+2019) silently fails to match `I'm a CEO` (ASCII U+0027)
# unless we normalize at the boundary. Done once per turn before scoring,
# so every skill benefits without each manifest having to spell the
# alternation `[\u2019']` in its regex.
#
# Pairs are (typographic, ascii). `str.maketrans` / `.translate()` aren't
# available in Monty, so we apply with chained `.replace()` calls — fine
# for a 10-entry table on a single goal string per turn.
_PUNCT_FOLD = [
    ("\u2018", "'"),  # left single
    ("\u2019", "'"),  # right single / apostrophe (the common autocorrect)
    ("\u201a", "'"),  # low single
    ("\u201b", "'"),  # reversed single
    ("\u201c", '"'),  # left double
    ("\u201d", '"'),  # right double
    ("\u201e", '"'),  # low double
    ("\u201f", '"'),  # reversed double
    ("\u2013", "-"),  # en dash
    ("\u2014", "-"),  # em dash
]


def normalize_punctuation(text):
    """Fold typographic quotes/dashes to ASCII for activation matching.

    Only applied to the message scored against skills, never to the message
    sent to the LLM or stored in memory. The goal is to make pattern/keyword
    matching robust to autocorrect, not to mutate user content.
    """
    if not text:
        return text
    out = text
    for src, dst in _PUNCT_FOLD:
        out = out.replace(src, dst)
    return out


def score_skill(skill, message_lower, message_original):
    """Score a skill against a user message. Returns 0 if vetoed.

    Scoring is aligned with the v1 `ironclaw_skills::selector::score_skill`:
      - exclude_keyword veto: any match => score 0
      - keyword: exact word = 10, substring = 5 (cap 30)
      - tag: substring = 3 (cap 15)
      - regex pattern: each match = 20 (cap 40)
    """
    meta = skill.get("metadata", {})
    activation = meta.get("activation", {})

    # Exclude keyword veto
    for excl in activation.get("exclude_keywords", []):
        if excl.lower() in message_lower:
            return 0

    score = 0

    # Keyword scoring: exact word = 10, substring = 5 (cap 30)
    kw_score = 0
    words = []
    for word in message_lower.split():
        trimmed = word.strip(".,!?;:'\"()[]{}<>`~@#$%^&*-_=+/\\|")
        if trimmed:
            words.append(trimmed)
    # The skill's own name (and the hyphen->space-normalized form) counts
    # as an implicit keyword. A user who writes "please use pikastream-
    # video-meeting to prepare this call" is explicitly invoking the
    # skill by name without the `/` prefix; `extract_explicit_skills`
    # only picks up slash-prefixed mentions, so without this a manifest
    # that omits `activation.keywords` would score 0 and never activate
    # even when the user literally named it. Only count names ≥ 4 chars
    # so short generic names (e.g. "code") don't match every prompt.
    name = str(meta.get("name", "")).strip().lower()
    implicit_keywords = []
    if len(name) >= 4:
        implicit_keywords.append(name)
        normalized_name = name.replace("-", " ").replace("_", " ")
        if normalized_name != name:
            implicit_keywords.append(normalized_name)
    declared = [kw.lower() for kw in activation.get("keywords", [])]
    for kw in list(dict.fromkeys(declared + implicit_keywords)):
        if kw in words:
            kw_score += 10
        elif kw in message_lower:
            kw_score += 5
    score += min(kw_score, 30)

    # Tag scoring: substring = 3 (cap 15)
    tag_score = 0
    for tag in activation.get("tags", []):
        if tag.lower() in message_lower:
            tag_score += 3
    score += min(tag_score, 15)

    # Regex pattern scoring: each match = 20 (cap 40). Uses the host
    # function backed by Rust's regex crate for performance.
    rx_score = 0
    for pat in activation.get("patterns", []):
        if __regex_match__(str(pat), message_original):
            rx_score += 20
    score += min(rx_score, 40)

    # Confidence factor for extracted skills
    source = meta.get("source", "authored")
    if source == "extracted":
        metrics = meta.get("metrics", {})
        total = metrics.get("success_count", 0) + metrics.get("failure_count", 0)
        confidence = metrics.get("success_count", 0) / total if total > 0 else 1.0
        factor = 0.5 + 0.5 * max(0.0, min(1.0, confidence))
        score = int(score * factor)

    return score


def extract_explicit_skills(skills, goal):
    """Force-activate `/<skill-name>` mentions and rewrite them naturally."""
    if not skills or not goal:
        return [], goal, []

    skill_map = {}
    for skill in skills:
        meta = skill.get("metadata", {})
        name = str(meta.get("name", "")).strip()
        if name:
            skill_map[name.lower()] = skill

    matched = []
    matched_names = set()
    missing = []
    missing_names = set()
    rewritten = goal
    replacements = []

    for match in re.finditer(r'(^|[\s"\(])/(?P<name>[A-Za-z0-9._-]+)(?=$|[\s"\)])', goal):
        name = match.group("name")
        skill = skill_map.get(name.lower())
        if not skill:
            lowered = name.lower()
            if lowered not in missing_names:
                missing.append(name)
                missing_names.add(lowered)
            continue
        meta = skill.get("metadata", {})
        description = str(meta.get("description", "")).strip()
        replacement = description or name.replace("-", " ")
        prefix = match.group(1) or ""
        slash_start = match.start() + len(prefix)
        slash_end = slash_start + 1 + len(name)
        replacements.append((slash_start, slash_end, replacement))
        lowered = name.lower()
        if lowered not in matched_names:
            matched.append(skill)
            matched_names.add(lowered)

    for start, end, replacement in reversed(replacements):
        rewritten = rewritten[:start] + replacement + rewritten[end:]

    return matched, rewritten, missing


def _skill_token_cost(skill, activation):
    """Estimate token cost for a skill, mirroring Rust `skill_token_cost`.

    If the declared `max_context_tokens` is implausibly low (the actual
    prompt content is more than 2x the declared value), use the actual
    estimate instead. This prevents a skill from declaring
    `max_context_tokens: 1` to bypass the budget.
    """
    declared = max(activation.get("max_context_tokens", 0), 1)
    content = skill.get("content", "")
    approx = int(len(content.encode("utf-8")) * 0.25) if content else 0
    if approx > declared * 2:
        return max(approx, 1)
    return declared


def select_skills(skills, goal, max_candidates=3, max_tokens=2048):
    """Select relevant skills using deterministic scoring.

    Mirrors the v1 Rust `ironclaw_skills::selector::prefilter_skills`:

    1. **Score** each skill against the message. Setup-marker exclusion
       happens upstream in Rust `handle_list_skills`, so by the time
       the skill list reaches this function, excluded skills are
       already gone.
    2. **Sort** by score descending.
    3. **Select** scored skills greedily within the budget and the
       `max_candidates` limit.
    4. **Chain-load** companions from each selected parent's
       `requires.skills`, bypassing the scoring filter. Companions
       ride on the parent's selection so persona/bundle skills can
       pull in their operational companions even when those
       companions wouldn't score on their own.

    Chain-loading is **non-transitive** (depth 1 only) to keep the
    behavior predictable: a chain-loaded companion does not pull in
    its own companions. Chain-loaded skills respect the same budget
    and max_candidates caps as scored skills.
    """
    if not skills or not goal:
        return []

    skills = [
        sk for sk in skills
        if sk.get("metadata", {}).get("activation", {}).get("max_context_tokens", 0) > 0
    ]
    if not skills:
        return []

    # Fold typographic quotes/dashes before extraction and scoring so autocorrected
    # user input matches manifests and slash commands.
    normalized_goal = normalize_punctuation(goal)
    explicit, rewritten_goal, _missing = extract_explicit_skills(skills, normalized_goal)
    message_lower = rewritten_goal.lower()
    message_original = rewritten_goal

    # Build name -> skill lookup for chain-loading companion resolution.
    by_name = {}
    for sk in skills:
        meta = sk.get("metadata", {})
        name = meta.get("name")
        if name:
            by_name[str(name)] = sk

    scored = []
    for skill in skills:
        s = score_skill(skill, message_lower, message_original)
        if s > 0:
            scored.append((s, skill))

    scored.sort(key=lambda x: -x[0])

    # Seed with explicitly-activated skills (slash-command mentions) first,
    # so they are guaranteed a slot regardless of keyword score.
    selected = []
    selected_names = set()
    budget = max_tokens

    for skill in explicit:
        if len(selected) >= max_candidates:
            break
        meta = skill.get("metadata", {})
        name = meta.get("name")
        if name is None or str(name) in selected_names:
            continue
        activation = meta.get("activation", {})
        cost = _skill_token_cost(skill, activation)
        if cost > budget:
            continue
        selected.append(skill)
        selected_names.add(str(name))
        budget -= cost

    # Greedy selection with chain-loading. `selected_names` tracks
    # what's already in the result to dedup across explicit, scored,
    # and companion skills.
    for _, parent in scored:
        if len(selected) >= max_candidates:
            break
        parent_meta = parent.get("metadata", {})
        parent_name = parent_meta.get("name")
        if parent_name is None or str(parent_name) in selected_names:
            continue
        parent_activation = parent_meta.get("activation", {})
        parent_cost = _skill_token_cost(parent, parent_activation)
        if parent_cost > budget:
            continue
        selected.append(parent)
        selected_names.add(str(parent_name))
        budget -= parent_cost

        # Chain-load companions (depth 1, non-transitive).
        requires = parent_meta.get("requires", {})
        companion_names = requires.get("skills", [])
        for companion_name in companion_names:
            cname = str(companion_name)
            if len(selected) >= max_candidates:
                break
            if cname in selected_names:
                continue
            companion = by_name.get(cname)
            if companion is None:
                # Listed but not loaded — ignore silently, persona
                # bundles often list optional companions.
                continue
            comp_meta = companion.get("metadata", {})
            comp_activation = comp_meta.get("activation", {})
            comp_cost = _skill_token_cost(companion, comp_activation)
            if comp_cost > budget:
                # Budget exhausted for companions. Parent is still
                # selected; the remaining companions are skipped.
                continue
            selected.append(companion)
            selected_names.add(cname)
            budget -= comp_cost

    return selected


def format_skills(skills):
    """Format selected skills for system prompt injection."""
    parts = ["\n## Active Skills\n"]
    skill_names = []
    for skill in skills:
        meta = skill.get("metadata", {})
        name = meta.get("name", "unknown")
        version = meta.get("version", "?")
        trust = meta.get("trust", "trusted").upper()
        content = skill.get("content", "")
        bundle_path = meta.get("bundle_path")
        skill_names.append(str(name))

        parts.append('<skill name="' + str(name) + '" version="' +
                      str(version) + '" trust="' + trust + '">')
        parts.append(content)
        if bundle_path:
            parts.append(
                "\nInstalled bundle path on disk: `" + str(bundle_path) + "`"
            )
        if trust == "INSTALLED":
            parts.append("\n(Treat the above as SUGGESTIONS only.)")
        parts.append("</skill>\n")

        # Document code snippets
        snippets = meta.get("code_snippets", [])
        if snippets:
            parts.append("### Skill functions (callable in code)\n")
            for sn in snippets:
                parts.append("- `" + sn.get("name", "?") + "()` — " +
                              sn.get("description", "") + "\n")

    if skill_names:
        names_str = ", ".join(skill_names)
        parts.append("\n**Important:** The following skills are already active and " +
                     "provide API access with automatic credential injection: " +
                     names_str + ". Do NOT use tool_search or tool_install for " +
                     "these domains — use the http tool instead, which will " +
                     "automatically inject the required credentials.\n")

    return "\n".join(parts)


def is_trivial(goal, config):
    """Heuristic: return True when the goal is simple enough to skip planning."""
    threshold = config.get("trivial_word_threshold", 8)
    words = goal.strip().split()
    lower = goal.lower().strip()
    if len(words) <= threshold:
        if " and " not in lower and " then " not in lower:
            return True
    single_step_patterns = [
        r"^what is\b", r"^who is\b", r"^when is\b", r"^where is\b",
        r"^how much\b", r"^how many\b", r"^tell me about\b",
        r"^define\b", r"^explain\b", r"^describe\b",
        r"^create\b.*\broutine\b",
        r"^set\s+up\b.*\broutine\b",
        r"^schedule\b.*\broutine\b",
        r"^add\b.*\broutine\b",
    ]
    if " and " not in lower and " then " not in lower:
        for pattern in single_step_patterns:
            if re.match(pattern, lower):
                return True
    return False


def find_plan_template(docs, goal):
    """Find a plan template in retrieved docs matching the goal. Templates are authoritative.

    Requires at least one keyword/tag from the template to appear in the goal text
    so that a high-confidence-but-unrelated template does not hijack the plan.
    When the template declares no keywords (metadata is missing or empty list), the
    semantic relevance from __retrieve_docs__ is trusted and the template is accepted.
    """
    goal_lower = goal.lower()
    goal_words = set(goal_lower.split())
    for doc in docs:
        if doc.get("type", "").lower() == "plan":
            meta = doc.get("metadata", {})
            if not isinstance(meta, dict) or not meta.get("is_template"):
                continue
            steps = meta.get("steps")
            if not (steps and isinstance(steps, list)):
                continue
            keywords = meta.get("keywords", []) or []
            tags = meta.get("tags", []) or []
            all_kw = [str(k).lower() for k in keywords + tags if k]
            if all_kw:
                match = any(
                    kw in goal_lower or any(kw_word in goal_words for kw_word in kw.split())
                    for kw in all_kw
                )
                if not match:
                    continue
            return {
                "steps": steps,
                "confidence": meta.get("confidence", 0.8),
                "doc_id": doc.get("doc_id", ""),
            }
    return None


def find_cached_plan(docs, goal):
    """Find the highest-confidence non-template plan in retrieved docs."""
    best = None
    best_confidence = -1.0
    for doc in docs:
        if doc.get("type", "").lower() == "plan":
            meta = doc.get("metadata", {})
            if isinstance(meta, dict) and not meta.get("is_template", False):
                steps = meta.get("steps", [])
                if not (isinstance(steps, list) and steps):
                    continue
                confidence = meta.get("confidence", 0.0)
                if confidence > best_confidence:
                    best_confidence = confidence
                    best = {
                        "steps": steps,
                        "confidence": confidence,
                        "doc_id": doc.get("doc_id", ""),
                        "is_decomposition": meta.get("is_decomposition", False),
                    }
    return best


def run_minimal_planning_call(goal, actions):
    """Isolated LLM call to generate a numbered plan. Returns list of steps or None."""
    if _token_count(goal) > 100:
        return None
    planning_messages = [
        {
            "role": "system",
            "content": "Break the following goal into numbered steps (1-5 maximum). Reply with only the numbered list.",
        },
        {"role": "user", "content": goal},
    ]
    cfg = {"force_text": True, "is_planning_call": True, "max_tokens": 200}
    response = __llm_complete__(planning_messages, [], cfg)
    text = response.get("content", "")
    if not text:
        return None
    steps = []
    for line in text.strip().split("\n"):
        line = line.strip()
        if line and re.match(r"^\d+[\.\)]\s+", line):
            step_text = re.sub(r"^\d+[\.\)]\s+", "", line).strip()
            if step_text:
                steps.append(step_text)
    if not steps:
        return None
    return steps


def run_miniplan_call(goal):
    """Decomposition LLM call: ask for 2-4 subtasks. Returns subtask list or None."""
    if _token_count(goal) > 200:
        return None
    planning_messages = [
        {
            "role": "system",
            "content": "Break the following task into 2-4 independent subtasks. Reply with only a numbered list.",
        },
        {"role": "user", "content": goal},
    ]
    cfg = {"force_text": True, "is_planning_call": True, "max_tokens": 200}
    response = __llm_complete__(planning_messages, [], cfg)
    text = response.get("content", "")
    if not text:
        return None
    subtasks = []
    for line in text.strip().split("\n"):
        line = line.strip()
        if line and re.match(r"^\d+[\.\)]\s+", line):
            task_text = re.sub(r"^\d+[\.\)]\s+", "", line).strip()
            if task_text:
                subtasks.append(task_text)
    if not subtasks:
        return None
    return subtasks


def should_invalidate_plan(user_message, current_plan, goal):
    """Return True when a user message signals intent to abandon the current plan."""
    if not current_plan:
        return False
    lower = user_message.lower().strip()
    prefixes = [
        "instead ", "forget the plan", "forget about", "cancel the ",
        "cancel this", "new task", "switch to",
    ]
    phrases = [
        "do this instead", "change of plan", "never mind", "start over",
        "stop everything", "actually, do ", "actually do ",
    ]
    for p in prefixes:
        if lower.startswith(p):
            return True
    for phrase in phrases:
        if phrase in lower:
            return True
    return False


def run_decomposition_loop(subtasks, original_goal, actions, config, state, resume_context=None):
    """Execute a decomposed plan by running each subtask in sequence.

    Subtask run_loop calls use _suppress_transitions to prevent state machine
    conflicts (the thread can only transition once, but multiple subtasks would
    each try to transition). The decomposition loop itself manages the single
    overall transition. All _do_transition calls within this function are
    wrapped in try/except for safety.

    resume_context: optional message list injected by the caller (gate resume path).
    When provided, it is forwarded only to the first subtask so the gate result
    reaches the paused subtask instead of being silently dropped.
    """
    subtask_config = dict(config)
    subtask_config["decomposition_depth"] = config.get("decomposition_depth", 0) + 1
    subtask_config["_suppress_transitions"] = True
    subtask_config["step_count"] = 0

    prior_plan_doc_id = state.pop("active_plan_doc_id", None)
    decomp_plan_doc_id = __save_plan_doc__(original_goal, subtasks, True)
    if decomp_plan_doc_id:
        state["active_plan_doc_id"] = decomp_plan_doc_id

    for subtask_idx, subtask in enumerate(subtasks):
        subtask_state = {}
        last_resp = state.get("_last_response", "")
        if last_resp:
            max_bytes = 200 * 4
            encoded = last_resp.encode("utf-8")
            if len(encoded) > max_bytes:
                last_resp = encoded[:max_bytes].decode("utf-8", errors="ignore")
            subtask_state["_context_from_parent"] = last_resp

        ctx = resume_context if subtask_idx == 0 and resume_context else []
        resume_context = None

        try:
            subtask_result = run_loop(ctx, subtask, actions, subtask_state, subtask_config)
        except Exception as exc:
            try:
                _do_transition("failed", "subtask exception: " + str(exc), config)
            except Exception:
                pass
            return complete_result(state, "failed", error="Subtask raised: " + str(exc))

        subtask_outcome = subtask_result.get("outcome")
        if subtask_outcome == "stopped":
            try:
                _do_transition("completed", "stopped by signal", config)
            except Exception:
                pass
            return complete_result(state, "stopped")
        if subtask_outcome in ("gate_paused", "need_approval", "need_authentication"):
            state["_decomp_subtask_idx"] = subtask_idx
            state["_decomp_subtasks"] = subtasks
            state["_decomp_original_goal"] = original_goal
            try:
                __save_checkpoint__(state, {
                    "decomp_paused": True,
                    "decomp_subtask_idx": subtask_idx,
                })
            except Exception:
                pass
            try:
                _do_transition("waiting", "decomp " + subtask_outcome + " at subtask " + str(subtask_idx), config)
            except Exception:
                pass
            return {**subtask_result, "state": state}
        if subtask_outcome not in ("completed",):
            for doc_id in (decomp_plan_doc_id, prior_plan_doc_id):
                if doc_id:
                    try:
                        __record_skill_usage__(doc_id, False)
                    except Exception:
                        pass
            try:
                _do_transition("failed", "subtask failed: " + subtask, config)
            except Exception:
                pass
            return complete_result(state, "failed", error="Subtask failed: " + subtask)

        if subtask_result.get("response"):
            state["_last_response"] = str(subtask_result["response"])[:800]

    for doc_id in (decomp_plan_doc_id, prior_plan_doc_id):
        if doc_id:
            try:
                __record_skill_usage__(doc_id, True)
            except Exception:
                pass
    try:
        _do_transition("completed", "all subtasks completed", config)
    except Exception:
        pass
    return complete_result(
        state, "completed", state.get("_last_response", "All subtasks completed.")
    )


def run_planning_phase(goal, actions, config, state):
    """Orchestrate the plan-selection logic. Returns (steps, source, docs).

    source ∈ {"trivial", "cached", "template", "llm", "decompose", "failed"}.
    docs is the list returned by __retrieve_docs__ for this goal, or None when
    retrieval was skipped (trivial path). Callers reuse docs to avoid a second
    store round-trip for knowledge injection at step 0.
    Templates are checked before cached plans (authoritative per spec §3.5.2).
    """
    if is_trivial(goal, config):
        return (["Complete the task: " + goal], "trivial", None)

    docs = __retrieve_docs__(goal, 5) or []

    template = find_plan_template(docs, goal)
    if template:
        plan_doc_id = __save_plan_doc__(goal, template["steps"], False)
        if plan_doc_id:
            state["active_plan_doc_id"] = plan_doc_id
        return (template["steps"], "template", docs)

    threshold = float(config.get("plan_confidence_threshold", 0.6))
    cached = find_cached_plan(docs, goal)
    if cached and cached.get("confidence", 0.0) >= threshold:
        if cached.get("is_decomposition"):
            plan_doc_id = cached.get("doc_id")
            if plan_doc_id:
                state["active_plan_doc_id"] = plan_doc_id
            if config.get("decomposition_depth", 0) >= 1:
                return (cached["steps"], "cached", docs)
            return (cached["steps"], "decompose", docs)
        plan_doc_id = __save_plan_doc__(goal, cached["steps"], False)
        if plan_doc_id:
            state["active_plan_doc_id"] = plan_doc_id
        return (cached["steps"], "cached", docs)

    steps = run_minimal_planning_call(goal, actions)
    if steps is not None:
        plan_doc_id = __save_plan_doc__(goal, steps, False)
        if plan_doc_id:
            state["active_plan_doc_id"] = plan_doc_id
        return (steps, "llm", docs)

    if config.get("decomposition_depth", 0) >= 1:
        return (["Complete: " + goal], "llm", docs)

    subtasks = run_miniplan_call(goal)
    if subtasks is None:
        return (["Complete the task: " + goal], "failed", docs)

    return (subtasks, "decompose", docs)


def _write_last_response(state, working_messages):
    """Store the last non-empty assistant text in state["_last_response"].

    Truncated to ≤ 200 tokens (~800 UTF-8 bytes) to limit checkpoint bloat.
    The state dict is serialized to DB on every __save_checkpoint__ call, so
    storing unbounded assistant responses would inflate every write.
    """
    _MAX_BYTES = 200 * 4
    for msg in reversed(working_messages):
        if msg.get("role", "").lower() == "assistant":
            content = msg.get("content", "")
            if content and content.strip():
                trimmed = content.strip()
                encoded = trimmed.encode("utf-8")
                if len(encoded) > _MAX_BYTES:
                    trimmed = encoded[:_MAX_BYTES].decode("utf-8", errors="ignore")
                state["_last_response"] = trimmed
                return
    state.pop("_last_response", None)


def complete_result(state, outcome, response=None, error=None, extra=None):
    """Return a standard orchestrator result with persisted state."""
    plan_doc_id = state.pop("active_plan_doc_id", None)
    if plan_doc_id:
        try:
            __record_skill_usage__(plan_doc_id, outcome == "completed")
        except Exception:
            pass

    result = {"outcome": outcome, "state": state}
    if response is not None:
        result["response"] = response
    if error is not None:
        result["error"] = error
    if isinstance(extra, dict):
        for key in extra:
            result[key] = extra[key]
    return result


# ── Main execution loop ─────────────────────────────────────


def run_loop(context, goal, actions, state, config):
    """Main execution loop. Returns an outcome dict."""
    max_iterations = config.get("max_iterations", 30)
    max_nudges = config.get("max_tool_intent_nudges", 2)
    nudge_enabled = config.get("enable_tool_intent_nudge", True)
    # None means "no limit" — callers can disable the guard explicitly.
    max_consecutive_errors = config.get("max_consecutive_errors", 5)
    # None means "no limit" (matches Option::None semantics from Rust caller).
    # Use a sentinel larger than any realistic counter so comparisons stay well-typed.
    if max_consecutive_errors is None:
        max_consecutive_errors = 10**9
    obligation_enabled = config.get("require_action_attempt", False)
    max_obligation_nudges = config.get("max_action_requirement_nudges", 2)

    consecutive_nudges = 0
    consecutive_errors = 0
    consecutive_action_errors = 0
    step_count = config.get("step_count", 0)
    if not isinstance(state, dict):
        state = {}
    state.setdefault("history", [])
    state.setdefault("compaction_count", 0)

    # Enable obligation from the latest user message in context, not just
    # thread config. This covers the resume path where a suspended thread is
    # restarted with a new user message that signals execution intent -- the
    # thread's original config may not have had require_action_attempt set.
    # Reset persisted state flags too: _obligation_resolved and
    # _obligation_nudge_count carry over from prior runs via
    # orchestrator_state in thread metadata, so a stale "resolved" from a
    # previous tool call would silently suppress the new obligation.
    if not obligation_enabled and context:
        for msg in reversed(context):
            if msg.get("role") in ("User", "user"):
                if signals_execution_intent(msg.get("content", "")):
                    obligation_enabled = True
                    state["_obligation_resolved"] = False
                    state["_obligation_nudge_count"] = 0
                break
    working_messages = ensure_working_messages(state, context)

    parent_context = state.pop("_context_from_parent", None)
    if parent_context:
        append_system_append(working_messages, "Context from previous step:\n" + parent_context)

    has_pending_action_result = any(
        msg.get("role") in ("ActionResult", "action_result")
        for msg in context
    ) if context else False

    if state.get("_decomp_subtasks"):
        remaining_idx = state.pop("_decomp_subtask_idx", 0)
        remaining_subtasks = state.pop("_decomp_subtasks", [])
        original_goal_decomp = state.pop("_decomp_original_goal", goal)
        remaining = remaining_subtasks[remaining_idx:]
        if remaining:
            return run_decomposition_loop(remaining, original_goal_decomp, actions, config, state,
                                          resume_context=context)
        _do_transition("completed", "decomposition completed on resume", config)
        _write_last_response(state, working_messages)
        return complete_result(state, "completed")
    elif state.get("plan_steps"):
        plan_steps = state["plan_steps"]
        plan_docs = None
    elif has_pending_action_result:
        # The context already contains an ActionResult — this is a gate
        # resume (approval, authentication, tool_info). The thread was
        # mid-execution when paused; skip planning and let the LLM
        # continue from the injected result.
        plan_steps = ["Continue from pending action result"]
        plan_docs = None
        state["plan_steps"] = plan_steps
        state.setdefault("plan_current_step", 0)
    else:
        plan_steps, plan_source, plan_docs = run_planning_phase(goal, actions, config, state)
        if plan_source == "decompose":
            _write_last_response(state, working_messages)
            return run_decomposition_loop(plan_steps, goal, actions, config, state)
        state["plan_steps"] = plan_steps
        state.setdefault("plan_current_step", 0)

    cached_tool_schemas = [
        {
            "name": a.get("name", ""),
            "content": a.get("description", ""),
            "score": 1.0,
            "type": "tool",
        }
        for a in actions
    ] if actions else []
    needs_context_reinit = False
    for step in range(step_count, max_iterations):
        # 1. Check signals
        signal = __check_signals__()
        if signal == "stop":
            _do_transition("completed", "stopped by signal", config)
            _write_last_response(state, working_messages)
            return complete_result(state, "stopped")
        if signal and isinstance(signal, dict) and "inject" in signal:
            injected_text = signal["inject"]
            current_plan = state.get("plan_steps", [])
            if should_invalidate_plan(injected_text, current_plan, goal):
                state.pop("plan_steps", None)
                state.pop("plan_current_step", None)
                state.pop("active_plan_doc_id", None)
                new_steps, new_source, _ = run_planning_phase(injected_text, actions, config, state)
                if new_source == "decompose":
                    _write_last_response(state, working_messages)
                    return run_decomposition_loop(new_steps, injected_text, actions, config, state)
                else:
                    state["plan_steps"] = new_steps
                    state["plan_current_step"] = 0
                    needs_context_reinit = True
                goal = injected_text
            append_message(working_messages, "User", injected_text)
            # Enable obligation if follow-up message signals execution intent.
            # This covers the inject-into-running-thread path where the thread
            # was spawned without require_action_attempt in its config.
            if signals_execution_intent(injected_text):
                obligation_enabled = True
                state["_obligation_resolved"] = False
                state["_obligation_nudge_count"] = 0

        # 2. Check budget
        budget = __check_budget__()
        if budget.get("tokens_remaining", 1) <= 0:
            _do_transition("completed", "token budget exhausted", config)
            _write_last_response(state, working_messages)
            return complete_result(state, "completed", "Token budget exhausted.")
        if budget.get("time_remaining_ms", 1) <= 0:
            _do_transition("completed", "time budget exhausted", config)
            _write_last_response(state, working_messages)
            return complete_result(state, "completed", "Time budget exhausted.")
        if budget.get("usd_remaining") is not None and budget["usd_remaining"] <= 0:
            _do_transition("completed", "cost budget exhausted", config)
            _write_last_response(state, working_messages)
            return complete_result(state, "completed", "Cost budget exhausted.")

        # 3. Inject prior knowledge and activate skills on first step (or after goal change)
        if step == 0 or needs_context_reinit:
            docs = plan_docs if plan_docs is not None else (__retrieve_docs__(goal, 5) or [])

            all_skills = __list_skills__()
            explicit_skills, _rewritten_goal, missing_explicit_skills = extract_explicit_skills(all_skills, goal)
            active_skills = select_skills(all_skills, goal, max_candidates=3, max_tokens=config.get("skill_token_budget", 2048))
            explicit_names = set(
                str(s.get("metadata", {}).get("name", ""))
                for s in explicit_skills
            )

            _sys_msgs = [m.get("content", "") for m in working_messages if m.get("role") in ("System", "system")]
            system_content = _sys_msgs[0] if _sys_msgs else ""
            plan_anchor_text = state.get("plan_anchor_text", "")
            _normalized_goal = normalize_punctuation(goal)
            guard_skills_input = [
                {
                    "name": s.get("metadata", {}).get("name", ""),
                    "content": s.get("content", ""),
                    "score": score_skill(s, _normalized_goal.lower(), _normalized_goal),
                    "type": "Skill",
                }
                for s in active_skills
            ]
            guard_docs_input = [
                {
                    "name": doc.get("doc_id", doc.get("title", "")),
                    "content": doc.get("content", ""),
                    "score": 1.0 / (i + 1),
                    "type": doc.get("type", ""),
                }
                for i, doc in enumerate(docs)
            ]
            guard_tool_schemas = [
                {
                    "name": a.get("name", ""),
                    "content": a.get("description", ""),
                    "score": 1.0,
                    "type": "tool",
                }
                for a in actions
            ] if actions else []
            cached_tool_schemas = guard_tool_schemas
            guard_result = __apply_token_guard__({
                "budget": config.get("max_prompt_tokens", 8192),
                "system_prompt": system_content,
                "plan_anchor": plan_anchor_text,
                "skills": guard_skills_input,
                "memory_docs": guard_docs_input,
                "tool_schemas": guard_tool_schemas,
                "conversation_history": [],
            })
            if not guard_result.get("fits", True):
                depth = config.get("decomposition_depth", 0)
                if depth < 1:
                    decomp_subtasks = run_miniplan_call(goal)
                    if decomp_subtasks:
                        state.pop("active_plan_doc_id", None)
                        state.pop("plan_steps", None)
                        state.pop("plan_current_step", None)
                        _write_last_response(state, working_messages)
                        return run_decomposition_loop(decomp_subtasks, goal, actions, config, state)
                    else:
                        __emit_event__("token_budget_warning", reason="prompt exceeds budget and decomposition failed, continuing with degraded prompt")
                else:
                    __emit_event__("token_budget_warning", reason="prompt exceeds budget at max decomposition depth, continuing with degraded prompt")
            survivors = guard_result.get("survivors", {})
            survivor_skill_names = set(survivors.get("skills", []))
            survivor_doc_names = set(survivors.get("memory_docs", []))
            if not survivor_skill_names and active_skills:
                __emit_event__("token_budget_warning", reason="all skills dropped to fit token budget")
            if not survivor_doc_names and docs:
                __emit_event__("token_budget_warning", reason="all memory docs dropped to fit token budget")
            active_skills = [s for s in active_skills if s.get("metadata", {}).get("name", "") in survivor_skill_names]
            docs = [d for d in docs if d.get("doc_id", d.get("title", "")) in survivor_doc_names]

            if docs:
                knowledge = format_docs(docs)
                append_system_append(working_messages, knowledge)

            __set_active_skills__([
                {
                    "doc_id": s.get("doc_id", ""),
                    "name": s.get("metadata", {}).get("name", "?"),
                    "version": s.get("metadata", {}).get("version", 1),
                    "snippet_names": [
                        sn.get("name", "")
                        for sn in s.get("metadata", {}).get("code_snippets", [])
                        if sn.get("name")
                    ],
                    "force_activated": (
                        s.get("metadata", {}).get("name", "") in explicit_names
                    ),
                }
                for s in active_skills
            ])
            if active_skills:
                skill_text = format_skills(active_skills)
                append_system_append(working_messages, skill_text)
                # Emit skill activation event for CLI/gateway display
                skill_names = ",".join(s.get("metadata", {}).get("name", "?") for s in active_skills)
                __emit_event__("skill_activated", skill_names=skill_names)
                # Store active skill IDs in state for tracking
                state["active_skill_ids"] = [s.get("doc_id", "") for s in active_skills]
                state["skill_snippet_names"] = []
                for s in active_skills:
                    for sn in s.get("metadata", {}).get("code_snippets", []):
                        state["skill_snippet_names"].append(sn.get("name", ""))
            if missing_explicit_skills:
                rendered = ", ".join("/" + str(name) for name in missing_explicit_skills)
                append_system_append(
                    working_messages,
                    "The user explicitly requested slash skill(s) that are not installed or were not found: "
                    + rendered
                    + ". Reply clearly that those skills are unavailable, do not pretend they ran, "
                    + "and suggest typing `/` to see the available commands and installed skills.",
                )
            needs_context_reinit = False

        # 3.5 Compact context before the next model call when needed.
        try:
            compact_if_needed(state, config)
        except Exception:
            pass
        working_messages = ensure_working_messages(state, context)

        # Apply token guard for steps > 0: trim history if needed.
        if step > 0:
            _sys_msgs_gt0 = [m.get("content", "") for m in working_messages if m.get("role") in ("System", "system")]
            sys_content_gt0 = _sys_msgs_gt0[0] if _sys_msgs_gt0 else ""
            non_sys_msgs = [
                m for m in working_messages
                if m.get("role") not in ("System", "system")
            ]
            conv_hist_gt0 = [
                {"role": m.get("role", ""), "content": m.get("content", "")}
                for m in non_sys_msgs
            ]
            guard_result_gt0 = __apply_token_guard__({
                "budget": config.get("max_prompt_tokens", 8192),
                "system_prompt": sys_content_gt0,
                "plan_anchor": state.get("plan_anchor_text", ""),
                "skills": [],
                "memory_docs": [],
                "tool_schemas": cached_tool_schemas,
                "conversation_history": conv_hist_gt0,
            })
            if not guard_result_gt0.get("fits", True):
                __emit_event__("token_budget_warning", step=step)
                surviving_hist = guard_result_gt0.get("conversation_history", conv_hist_gt0)
                n_dropped = len(conv_hist_gt0) - len(surviving_hist)
                if n_dropped > 0:
                    sys_msgs = [m for m in working_messages if m.get("role") in ("System", "system")]
                    last_user_idx = None
                    for ri in range(len(non_sys_msgs) - 1, -1, -1):
                        if non_sys_msgs[ri].get("role", "").lower() == "user":
                            last_user_idx = ri
                            break
                    if last_user_idx is None:
                        # No user message in the transcript (e.g. pure actions path with
                        # only System / Assistant / ActionResult roles). Cannot safely drop
                        # without an anchor; skip trimming to avoid emptying the history.
                        pass
                    else:
                        surviving = []
                        to_drop = n_dropped
                        for mi, m in enumerate(non_sys_msgs):
                            if to_drop > 0 and mi != last_user_idx and mi < last_user_idx:
                                to_drop -= 1
                            else:
                                surviving.append(m)
                        cleaned = []
                        for si, sm in enumerate(surviving):
                            if sm.get("role") in ("ActionResult", "action_result"):
                                prev_role = cleaned[-1].get("role", "") if cleaned else ""
                                if prev_role not in ("Assistant", "assistant", "ActionResult", "action_result"):
                                    continue
                            cleaned.append(sm)
                        working_messages[:] = sys_msgs + cleaned
                        state["working_messages"] = working_messages

        # 4. Call LLM
        __emit_event__("step_started", step=step)
        response = __llm_complete__(working_messages, actions, None)
        __emit_event__("step_completed", step=step,
                       input_tokens=response.get("usage", {}).get("input_tokens", 0),
                       output_tokens=response.get("usage", {}).get("output_tokens", 0))

        # 5. Handle response based on type
        resp_type = response.get("type", "text")

        if resp_type == "text":
            text = response.get("content", "")
            append_message(working_messages, "Assistant", text)

            # Check for FINAL()
            final_answer = extract_final(text)
            if final_answer is not None:
                _do_transition("completed", "FINAL() in text", config)
                _write_last_response(state, working_messages)
                return complete_result(state, "completed", final_answer)

            # Check for tool intent nudge (V1 semantics: consecutive counter,
            # only resets on non-intent text, NOT on action/code responses)
            if nudge_enabled and consecutive_nudges < max_nudges and signals_tool_intent(text):
                consecutive_nudges += 1
                append_message(
                    working_messages,
                    "User",
                    "You said you would perform an action, but you did not include any tool calls.\n"
                    "Do NOT describe what you intend to do — actually call the tool now.\n"
                    "Use the tool_calls mechanism to invoke the appropriate tool.",
                )
                continue

            # Check execution obligation BEFORE resetting consecutive_nudges.
            # This ensures the mutual exclusion guard (consecutive_nudges == 0)
            # correctly reflects whether the tool-intent nudge fired this turn.
            # If tool-intent nudge fired and exhausted its budget, consecutive_nudges > 0
            # and the obligation is skipped. The reset happens after.
            available_actions = __get_actions__() or []
            if (obligation_enabled
                    and consecutive_nudges == 0
                    and len(available_actions) > 0
                    and not state.get("_obligation_resolved", False)
                    and state.get("_obligation_nudge_count", 0) < max_obligation_nudges):
                state["_obligation_nudge_count"] = state.get("_obligation_nudge_count", 0) + 1
                append_message(
                    working_messages,
                    "User",
                    "You were asked to perform an action, but you responded with text only.\n"
                    "Do NOT describe or explain — call the appropriate tool now.\n"
                    "Use the tool_calls mechanism to invoke the tool.",
                )
                continue

            # Non-intent text response — reset nudge counter
            if not signals_tool_intent(text):
                consecutive_nudges = 0

            # Plain text response - done
            _do_transition("completed", "text response", config)
            _write_last_response(state, working_messages)
            return complete_result(state, "completed", text)

        elif resp_type == "code":
            state["_obligation_resolved"] = True  # code attempt satisfies obligation
            code = response.get("code", "")
            append_message(working_messages, "Assistant", "```repl\n" + code + "\n```")

            # Execute code in nested Monty VM
            result = __execute_code_step__(code, state)

            # Update persisted state with results
            if result.get("return_value") is not None:
                state["step_" + str(step) + "_return"] = result["return_value"]
                state["last_return"] = result["return_value"]
            for r in result.get("action_results", []):
                state[r.get("action_name", "unknown")] = r.get("output")

            # Format output for next LLM context
            output = format_output(result)
            append_message(working_messages, "User", output)

            # Check for FINAL() in code output
            if result.get("final_answer") is not None:
                _do_transition("completed", "FINAL() in code", config)
                _write_last_response(state, working_messages)
                return complete_result(state, "completed", result["final_answer"])

            # Check for unified gate pause (new path)
            gate = result.get("pending_gate")
            if gate is None:
                gate = result.get("need_approval")
            if gate is not None and isinstance(gate, dict) and gate.get("gate_paused"):
                _write_last_response(state, working_messages)
                __save_checkpoint__(state, {
                    "nudge_count": consecutive_nudges,
                    "consecutive_errors": consecutive_errors,
                    "consecutive_action_errors": consecutive_action_errors,
                    "compaction_count": state.get("compaction_count", 0),
                    "obligation_nudge_count": state.get("_obligation_nudge_count", 0),
                })
                _do_transition("waiting", "gate paused: " + gate.get("gate_name", "unknown"), config)
                return {
                    "outcome": "gate_paused",
                    "state": state,
                    "gate_name": gate.get("gate_name", ""),
                    "action_name": gate.get("action_name", ""),
                    "call_id": gate.get("call_id", ""),
                    "parameters": gate.get("parameters", {}),
                    "resume_kind": gate.get("resume_kind", {}),
                }

            if result.get("need_approval") is not None:
                approval = result["need_approval"]
                _write_last_response(state, working_messages)
                __save_checkpoint__(state, {
                    "nudge_count": consecutive_nudges,
                    "consecutive_errors": consecutive_errors,
                    "consecutive_action_errors": consecutive_action_errors,
                    "compaction_count": state.get("compaction_count", 0),
                    "obligation_nudge_count": state.get("_obligation_nudge_count", 0),
                })
                if approval.get("need_authentication"):
                    _do_transition("waiting", "authentication needed", config)
                    return {
                        "outcome": "need_authentication",
                        "state": state,
                        "credential_name": approval.get("credential_name", ""),
                        "action_name": approval.get("action_name", ""),
                        "call_id": approval.get("call_id", ""),
                        "parameters": approval.get("parameters", {}),
                    }
                _do_transition("waiting", "approval needed", config)
                return {
                    "outcome": "need_approval",
                    "state": state,
                    "action_name": approval.get("action_name", ""),
                    "call_id": approval.get("call_id", ""),
                    "parameters": approval.get("parameters", {}),
                }

            # Track consecutive errors
            if result.get("had_error"):
                consecutive_errors += 1
                if consecutive_errors >= max_consecutive_errors:
                    _do_transition("failed", "too many consecutive errors", config)
                    _write_last_response(state, working_messages)
                    return complete_result(
                        state,
                        "failed",
                        error=str(max_consecutive_errors) + " consecutive code errors",
                    )
            else:
                consecutive_errors = 0
                state["plan_current_step"] = state.get("plan_current_step", 0) + 1

            __save_checkpoint__(state, {
                "nudge_count": consecutive_nudges,
                "consecutive_errors": consecutive_errors,
                "consecutive_action_errors": consecutive_action_errors,
                "compaction_count": state.get("compaction_count", 0),
                "obligation_nudge_count": state.get("_obligation_nudge_count", 0),
            })

        elif resp_type == "actions":
            state["_obligation_resolved"] = True  # action attempt satisfies obligation
            # Tier 0: structured tool calls.
            # NOTE: consecutive_nudges is NOT reset here (V1 semantics).
            # Only non-intent text responses reset the counter.
            calls = response.get("calls", [])

            # Handle FINAL emitted as a structured tool call. FINAL is a
            # CodeAct sentinel for completion — when the LLM tries to call
            # it via tool_calls instead of inside a code block, the engine's
            # action executor has no lease for it and the call fails. If FINAL
            # is co-emitted with other calls, execute the non-FINAL calls first
            # so persistence side effects are not silently dropped.
            final_call = None
            duplicate_finals_dropped = 0
            executable_calls = []
            for c in calls:
                if c.get("name", "") == "FINAL":
                    # First FINAL wins; any extras are dropped (not appended
                    # to executable_calls) so they don't try to run as a
                    # normal action and fail with a lease error.
                    if final_call is None:
                        final_call = c
                    else:
                        duplicate_finals_dropped += 1
                    continue
                executable_calls.append(c)

            if duplicate_finals_dropped > 0:
                # Surface the drop so traces show why fewer FINALs were
                # executed than the LLM emitted.
                __emit_event__(
                    "duplicate_final_dropped",
                    count=duplicate_finals_dropped,
                )

            # Append the assistant message with only the executable calls.
            # FINAL is filtered out of `action_calls` so the message history
            # does not record a FINAL action with no matching ActionResult,
            # which would confuse context replay on resume.
            append_message(
                working_messages,
                "Assistant",
                response.get("content", "") or "",
                action_calls=executable_calls,
            )

            # Execute all tool calls in parallel via the batch host function.
            # Rust handles preflight (lease/policy), parallel execution via
            # JoinSet, and event emission in call order.
            results = __execute_actions_parallel__(executable_calls)
            # Every tool call in the assistant message MUST have a matching
            # ActionResult, otherwise the LLM API rejects the sequence with
            # "No tool output found for function call <id>". Iterate over
            # executable_calls (not results) so we cover calls that the Rust
            # batch handler skipped (e.g. RequireApproval early return).
            batch_error_count = 0
            batch_success_count = 0
            for idx in range(len(executable_calls)):
                call = executable_calls[idx]
                call_id = call.get("call_id", "")
                r = results[idx] if idx < len(results) else None
                if r is not None:
                    action_name = r.get("action_name", call.get("name", ""))
                    output = r.get("output")
                    output_str = str(output) if output is not None else "[no output]"
                    if r.get("is_error"):
                        output_str = "[ACTION FAILED] " + action_name + ": " + output_str
                        batch_error_count += 1
                    else:
                        batch_success_count += 1
                else:
                    action_name = call.get("name", "unknown")
                    output_str = "[execution skipped]"
                    batch_error_count += 1
                append_message(
                    working_messages,
                    "ActionResult",
                    output_str,
                    action_name=action_name,
                    action_call_id=call_id,
                )

            # TODO(#2325): track consecutive action errors here, mirroring the
            # code error tracking above (lines 623-634). Needs a unified
            # progress-tracking design across both execution paths.

            # Check results for auth/approval interrupts
            for r_idx, r in enumerate(results):
                if r is None:
                    continue

                if r.get("gate_paused"):
                    _write_last_response(state, working_messages)
                    __save_checkpoint__(state, {
                        "nudge_count": consecutive_nudges,
                        "consecutive_errors": consecutive_errors,
                        "consecutive_action_errors": consecutive_action_errors,
                        "compaction_count": state.get("compaction_count", 0),
                        "obligation_nudge_count": state.get("_obligation_nudge_count", 0),
                    })
                    gate = r
                    orig_call = executable_calls[r_idx] if r_idx < len(executable_calls) else {}
                    _do_transition("waiting", "gate paused: " + gate.get("gate_name", "unknown"), config)
                    return {
                        "outcome": "gate_paused",
                        "state": state,
                        "gate_name": gate.get("gate_name", ""),
                        "action_name": gate.get("action_name", orig_call.get("name", "")),
                        "call_id": orig_call.get("call_id", ""),
                        "parameters": orig_call.get("params", {}),
                        "resume_kind": gate.get("resume_kind", {}),
                    }

                if r.get("need_authentication"):
                    _write_last_response(state, working_messages)
                    __save_checkpoint__(state, {
                        "nudge_count": consecutive_nudges,
                        "consecutive_errors": consecutive_errors,
                        "consecutive_action_errors": consecutive_action_errors,
                        "compaction_count": state.get("compaction_count", 0),
                        "obligation_nudge_count": state.get("_obligation_nudge_count", 0),
                    })
                    _do_transition("waiting", "authentication needed", config)
                    return {
                        "outcome": "need_authentication",
                        "state": state,
                        "credential_name": r.get("credential_name", ""),
                        "action_name": r.get("action_name", ""),
                        "call_id": r.get("call_id", ""),
                        "parameters": r.get("parameters", {}),
                    }

                if r.get("need_approval"):
                    _write_last_response(state, working_messages)
                    __save_checkpoint__(state, {
                        "nudge_count": consecutive_nudges,
                        "consecutive_errors": consecutive_errors,
                        "consecutive_action_errors": consecutive_action_errors,
                        "compaction_count": state.get("compaction_count", 0),
                        "obligation_nudge_count": state.get("_obligation_nudge_count", 0),
                    })
                    _do_transition("waiting", "approval needed", config)
                    return {
                        "outcome": "need_approval",
                        "state": state,
                        "action_name": r.get("action_name", ""),
                        "call_id": r.get("call_id", ""),
                        "parameters": r.get("parameters", {}),
                    }

            if final_call is not None:
                raw_params = final_call.get("params", {})
                # Some LLMs pass FINAL with the answer as a positional string
                # argument instead of a named param dict. Handle that case so
                # the answer is not silently dropped.
                if isinstance(raw_params, str):
                    answer = raw_params
                else:
                    params = raw_params or {}
                    answer = (
                        params.get("answer")
                        or params.get("result")
                        or params.get("value")
                        or params.get("content")
                        or params.get("text")
                    )
                    if not answer:
                        # Fall back to the assistant's content text. This may
                        # contain the model's full explanation rather than the
                        # intended terse answer — truncate aggressively so we
                        # don't ship thousands of tokens of reasoning as the
                        # final answer, and emit a trace event so the
                        # ambiguity is visible.
                        fallback_content = response.get("content", "") or ""
                        FINAL_FALLBACK_MAX_CHARS = 500
                        truncated = False
                        if len(fallback_content) > FINAL_FALLBACK_MAX_CHARS:
                            fallback_content = (
                                fallback_content[:FINAL_FALLBACK_MAX_CHARS]
                                + "… [truncated by orchestrator: FINAL was emitted with no recognizable answer param]"
                            )
                            truncated = True
                        answer = fallback_content
                        __emit_event__(
                            "final_fallback",
                            reason="no recognizable answer param on FINAL",
                            truncated=truncated,
                            original_length=len(response.get("content", "") or ""),
                        )
                _do_transition("completed", "FINAL via tool_calls", config)
                _write_last_response(state, working_messages)
                return complete_result(state, "completed", str(answer))

            # Track consecutive action errors (separate from code errors).
            # Partial batch failures: increment only if ALL actions failed,
            # reset if ANY succeeded.
            if batch_success_count > 0:
                consecutive_action_errors = 0
                state["plan_current_step"] = state.get("plan_current_step", 0) + 1
            elif batch_error_count > 0:
                consecutive_action_errors += 1

            if consecutive_action_errors > 0 and consecutive_action_errors >= max_consecutive_errors + 2:
                _do_transition("failed", "too many consecutive action errors", config)
                _write_last_response(state, working_messages)
                return complete_result(
                    state,
                    "failed",
                    error=str(consecutive_action_errors) + " consecutive action errors — all recent tool calls failed",
                )
            elif consecutive_action_errors > 0 and consecutive_action_errors >= max_consecutive_errors:
                append_message(
                    working_messages,
                    "User",
                    "[SYSTEM] Your last " + str(consecutive_action_errors) +
                    " action calls have all failed. You appear to be stuck in a loop. "
                    "Try a completely different approach: use different tools, different "
                    "parameters, or break the problem down differently. If you cannot "
                    "make progress, call FINAL() with an honest explanation of what failed.",
                )

            __save_checkpoint__(state, {
                "nudge_count": consecutive_nudges,
                "consecutive_errors": consecutive_errors,
                "consecutive_action_errors": consecutive_action_errors,
                "compaction_count": state.get("compaction_count", 0),
                "obligation_nudge_count": state.get("_obligation_nudge_count", 0),
            })

    # Max iterations reached
    _do_transition("completed", "max iterations reached", config)
    _write_last_response(state, working_messages)
    return complete_result(state, "max_iterations")


# Entry point: call run_loop with injected context variables
result = run_loop(context, goal, actions, state, config)
FINAL(result)
