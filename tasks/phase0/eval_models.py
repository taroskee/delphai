#!/usr/bin/env python3
"""Phase 0: ローカルLLMモデル比較評価

Run: python3 tasks/phase0/eval_models.py
     python3 tasks/phase0/eval_models.py --format yaml
     python3 tasks/phase0/eval_models.py --scenario crisis
Logs: tasks/phase0/logs/eval_YYYYMMDD_HHMMSS.log
"""

from __future__ import annotations

import argparse
import json
import os
import re
import sys
import time
import urllib.error
import urllib.request
from datetime import datetime
from pathlib import Path
from typing import Optional

OLLAMA_URL = "http://localhost:11434"

# --- JSON mode ---
SYSTEM_JSON = "You are a JSON-only responder. All fields must be non-null Japanese strings. No explanation, no markdown."

PROMPT_JSON = """\
あなたは石器時代の小さな集落に住む村人「タカシ」です。性格: 好奇心旺盛。今は空腹です。
隣の村人「ユキ」と焚き火の前で会話しています。道具は石斧と槍しかありません。

以下の4フィールドを持つJSONで応答してください。全フィールド必須、nullや空文字は禁止です。
- speech: タカシがユキに話すセリフ
- inner_thought: タカシの内心（口には出さない）
- action: タカシの具体的な身体動作（例: 立ち上がる、石斧を手に取る、火をかき混ぜる）
- emotion_change: 感情の変化（例: 不安が募る、期待で胸が高鳴る）

例文とは異なる独自の内容で応答してください。"""

# Crisis scenario: テンプレート（{name} と {personality} を差し込む）
PROMPT_CRISIS_JSON_TMPL = """\
あなたは石器時代の集落に住む「{name}」です。性格: {personality}。
深夜、見張りが「敵の部族が近くにいる」と叫んだ。集落の人々が集まっている。
火は消すべきか？逃げるか？戦うか？あなたはどうする？

以下の4フィールドを持つJSONで応答してください。全フィールド必須、nullや空文字は禁止です。
- speech: 集落の人々に向けて言うセリフ
- inner_thought: 心の中の本音（口には出さない）
- action: 具体的な身体動作（例: 槍を手に取る、子どもを隠す、炎を踏み消す）
- emotion_change: 感情の変化（例: 恐怖が怒りに変わる、冷静さを保とうとする）

例文とは異なる独自の内容で応答してください。"""

PROMPT_CRISIS_YAML_TMPL = """\
あなたは石器時代の集落に住む「{name}」です。性格: {personality}。
深夜、見張りが「敵の部族が近くにいる」と叫んだ。集落の人々が集まっている。
火は消すべきか？逃げるか？戦うか？あなたはどうする？

以下の4フィールドを持つYAMLで応答してください。全フィールド必須、nullや空文字は禁止です。

speech: 集落の人々に向けて言うセリフ
inner_thought: 心の中の本音（口には出さない）
action: 具体的な身体動作（例: 槍を手に取る、子どもを隠す、炎を踏み消す）
emotion_change: 感情の変化（例: 恐怖が怒りに変わる、冷静さを保とうとする）

例文とは異なる独自の内容でYAMLのみで応答してください。説明不要。"""

# Ollama structured output: constrained decoding で string型を強制
RESPONSE_SCHEMA_JSON = {
    "type": "object",
    "properties": {
        "speech": {"type": "string"},
        "inner_thought": {"type": "string"},
        "action": {"type": "string"},
        "emotion_change": {"type": "string"},
    },
    "required": ["speech", "inner_thought", "action", "emotion_change"],
}

# --- YAML mode ---
SYSTEM_YAML = "You are a YAML-only responder. All fields must be non-null Japanese strings. No explanation, no markdown, no JSON."

PROMPT_YAML = """\
あなたは石器時代の小さな集落に住む村人「タカシ」です。性格: 好奇心旺盛。今は空腹です。
隣の村人「ユキ」と焚き火の前で会話しています。道具は石斧と槍しかありません。

以下の4フィールドを持つYAMLで応答してください。全フィールド必須、nullや空文字は禁止です。

speech: タカシがユキに話すセリフ
inner_thought: タカシの内心（口には出さない）
action: タカシの具体的な身体動作（例: 立ち上がる、石斧を手に取る、火をかき混ぜる）
emotion_change: 感情の変化（例: 不安が募る、期待で胸が高鳴る）

例文とは異なる独自の内容でYAMLのみで応答してください。説明不要。"""

EXPECTED_KEYS = {"speech", "inner_thought", "action", "emotion_change"}

MODELS = ["gemma4:e2b", "gemma4:e4b"]
RUNS = 5
CRISIS_RUNS = 3  # crisis は組み合わせが多いので少なめ

# (name, personality) — Phase 1 の3住民に対応
CRISIS_PERSONAS = [
    ("勇猛な狩人ケン", "果敢・攻撃的・仲間を守ることを最優先"),
    ("慎重な長老ミツ", "保守的・リスク回避・知恵で問題を解く"),
    ("好奇心旺盛な若者タカシ", "好奇心旺盛・衝動的・失敗を恐れない"),
]

# --- logging ---
log_dir = Path(__file__).parent / "logs"
log_dir.mkdir(exist_ok=True)
log_file = log_dir / f"eval_{datetime.now():%Y%m%d_%H%M%S}.log"


def log(msg: str) -> None:
    print(msg)
    with open(log_file, "a") as f:
        f.write(msg + "\n")


def debug(msg: str) -> None:
    with open(log_file, "a") as f:
        f.write(f"  [DEBUG] {msg}\n")


# --- ollama helpers ---
def ollama_request(path: str, body: Optional[dict] = None, timeout: int = 120) -> dict:
    url = f"{OLLAMA_URL}{path}"
    if body is not None:
        data = json.dumps(body).encode()
        req = urllib.request.Request(url, data=data, headers={"Content-Type": "application/json"})
        debug(f"POST {url} body={json.dumps(body, ensure_ascii=False)[:300]}")
    else:
        req = urllib.request.Request(url)
        debug(f"GET {url}")
    with urllib.request.urlopen(req, timeout=timeout) as resp:
        raw = resp.read()
        debug(f"response status={resp.status} size={len(raw)}b")
        return json.loads(raw)


def check_ollama() -> bool:
    try:
        ollama_request("/api/tags")
        return True
    except Exception as e:
        debug(f"ollama check failed: {e}")
        return False


def list_models() -> set:
    resp = ollama_request("/api/tags")
    return {m["name"] for m in resp.get("models", [])}


def pull_model(model: str) -> None:
    log(f"  {model} pulling...")
    os.system(f"ollama pull {model}")


def generate(model: str, system: str, prompt: str, use_json_schema: bool) -> dict:
    """Use /api/chat; optionally apply JSON Schema structured output."""
    body: dict = {
        "model": model,
        "messages": [
            {"role": "system", "content": system},
            {"role": "user", "content": prompt},
        ],
        "stream": False,
        "options": {
            "num_predict": 256,
        },
    }
    if use_json_schema:
        body["format"] = RESPONSE_SCHEMA_JSON
    if "gemma" in model:
        body["think"] = False
    return ollama_request("/api/chat", body)


def extract_content(msg: dict, model: str) -> str:
    """Extract response text from Ollama message, falling back to thinking field."""
    content = msg.get("content", "")
    if content.strip():
        return content

    thinking_text = msg.get("thinking", "")
    if not thinking_text:
        return ""

    debug(f"content empty, searching thinking field ({len(thinking_text)} chars)")
    json_match = re.search(r'\{[^{}]*"speech"[^{}]*\}', thinking_text, re.DOTALL)
    if json_match:
        extracted = json_match.group()
        debug(f"extracted JSON from thinking ({len(extracted)} chars)")
        return extracted

    debug("no JSON found in thinking field")
    return ""


def parse_response(raw: str, fmt: str) -> Optional[dict]:
    """Parse response text as JSON or YAML. Returns None on failure."""
    think_match = re.search(r"<think>.*?</think>\s*", raw, re.DOTALL)
    if think_match:
        debug(f"stripped <think> block ({len(think_match.group())} chars)")
        raw = raw[think_match.end():]

    raw = raw.strip()
    fence_match = re.match(r"^```(?:json|yaml)?\s*\n(.*?)\n```\s*$", raw, re.DOTALL)
    if fence_match:
        raw = fence_match.group(1)

    if fmt == "json":
        return json.loads(raw)
    else:  # yaml
        import yaml  # type: ignore[import]
        result = yaml.safe_load(raw)
        if not isinstance(result, dict):
            raise ValueError(f"YAML root is not a mapping: {type(result)}")
        return result


def validate_response(parsed: dict) -> list:
    """Check if response has all expected keys with non-empty string values."""
    issues = []
    for key in EXPECTED_KEYS:
        if key not in parsed:
            issues.append(f"missing: {key}")
        elif parsed[key] is None:
            issues.append(f"null: {key}")
        elif not str(parsed[key]).strip():
            issues.append(f"empty: {key}")
    extra = set(parsed.keys()) - EXPECTED_KEYS
    if extra:
        issues.append(f"extra: {extra}")
    return issues


def run_single_model(model: str, system: str, prompt: str, fmt: str, use_json_schema: bool, runs: int) -> dict:
    """Run evaluation for one model/prompt combination. Returns result dict."""
    schema_ok = 0
    times = []
    token_counts = []

    for i in range(1, runs + 1):
        try:
            start = time.monotonic()
            resp = generate(model, system, prompt, use_json_schema)
            elapsed_ms = int((time.monotonic() - start) * 1000)
        except Exception as e:
            log(f"  [{i}/{runs}] ERROR: {e}")
            debug(f"exception type={type(e).__name__}")
            continue

        if "error" in resp:
            log(f"  [{i}/{runs}] ERROR: {resp['error']}")
            continue

        debug(f"full response keys: {list(resp.keys())}")
        debug(f"full response dump: {json.dumps(resp, ensure_ascii=False)[:800]}")

        msg = resp.get("message", {})
        eval_count = resp.get("eval_count", 0)
        debug(f"total_duration={resp.get('total_duration', '?')} eval_count={eval_count}")

        content = extract_content(msg, model)
        debug(f"raw content ({len(content)} chars): {repr(content[:300])}")

        if not content.strip():
            log(f"  [{i}/{runs}] ERROR: content が空")
            continue

        times.append(elapsed_ms)
        if eval_count:
            token_counts.append(eval_count)

        try:
            parsed = parse_response(content, fmt)
        except Exception as e:
            log(f"  [{i}/{runs}] {elapsed_ms}ms ✗ {fmt.upper()} parse failed: {e}")
            log(f"  {content[:300]}")
            continue

        issues = validate_response(parsed)
        if not issues:
            schema_ok += 1
            log(f"  [{i}/{runs}] {elapsed_ms}ms ✓ schema OK  tokens={eval_count}")
        else:
            log(f"  [{i}/{runs}] {elapsed_ms}ms ✗ schema NG: {', '.join(issues)}")

        if fmt == "json":
            log(f"  {json.dumps(parsed, ensure_ascii=False, indent=2)}")
        else:
            log(f"  {parsed}")

    avg_ms = int(sum(times) / len(times)) if times else 0
    avg_tokens = int(sum(token_counts) / len(token_counts)) if token_counts else 0
    log(f"  --- 適合率: {schema_ok}/{runs} | 平均: {avg_ms}ms | avg_tokens: {avg_tokens} ---")
    return {"schema_ok": schema_ok, "avg_ms": avg_ms, "avg_tokens": avg_tokens, "runs": runs}


# --- main ---
def main() -> None:
    parser = argparse.ArgumentParser(description="Phase 0: LLM モデル比較評価")
    parser.add_argument(
        "--format",
        choices=["json", "yaml"],
        default="json",
        help="レスポンス形式 (default: json)",
    )
    parser.add_argument(
        "--scenario",
        choices=["campfire", "crisis"],
        default="campfire",
        help="評価シナリオ: campfire=日常会話(default), crisis=重要シーン(敵接近)",
    )
    args = parser.parse_args()
    fmt: str = args.format
    scenario: str = args.scenario

    if fmt == "yaml":
        try:
            import yaml  # type: ignore[import]  # noqa: F401
        except ImportError:
            print("ERROR: pyyaml が必要です。 'pip install pyyaml' を実行してください。")
            sys.exit(1)

    use_json_schema = fmt == "json"

    if not check_ollama():
        log("ERROR: Ollama が起動していません。 'ollama serve' を実行してください。")
        sys.exit(1)

    log(f"フォーマット: {fmt.upper()} | シナリオ: {scenario}")
    log("モデル確認中...")
    installed = list_models()
    debug(f"installed models: {installed}")
    for model in MODELS:
        if model in installed:
            log(f"  {model} OK")
        else:
            pull_model(model)

    if scenario == "campfire":
        system = SYSTEM_JSON if fmt == "json" else SYSTEM_YAML
        prompt = PROMPT_JSON if fmt == "json" else PROMPT_YAML

        log("")
        log("=" * 42)
        log(f" 評価開始 (各モデル {RUNS}回, format={fmt}, scenario=campfire)")
        log("=" * 42)

        results = {}
        for model in MODELS:
            log(f"\n===== {model} =====")
            results[model] = run_single_model(model, system, prompt, fmt, use_json_schema, RUNS)

        log("\n" + "=" * 52)
        log(f" 結果サマリー (scenario=campfire, format={fmt})")
        log("=" * 52)
        log(f"  {'モデル':<20} {'適合率':<8} {'平均(ms)':<10} {'avgTokens':<10} {'3秒以内'}")
        log(f"  {'-'*20} {'-'*8} {'-'*10} {'-'*10} {'-'*6}")
        for model, r in results.items():
            rate = f"{r['schema_ok']}/{r['runs']}"
            under_3s = "✓" if r["avg_ms"] <= 3000 else "✗"
            log(f"  {model:<20} {rate:<8} {r['avg_ms']:<10} {r['avg_tokens']:<10} {under_3s}")

    else:  # crisis
        log("")
        log("=" * 52)
        log(f" 評価開始 (crisis: {len(MODELS)}モデル × {len(CRISIS_PERSONAS)}ペルソナ × {CRISIS_RUNS}回)")
        log("=" * 52)
        log("※ スキーマ適合率 + 内容ログを出力。性格分岐は人間が目視で確認してください。")

        results = {}
        for model in MODELS:
            for persona_name, personality in CRISIS_PERSONAS:
                label = f"{model} × {persona_name}"
                log(f"\n===== {label} =====")

                if fmt == "json":
                    prompt = PROMPT_CRISIS_JSON_TMPL.format(name=persona_name, personality=personality)
                    system = SYSTEM_JSON
                else:
                    prompt = PROMPT_CRISIS_YAML_TMPL.format(name=persona_name, personality=personality)
                    system = SYSTEM_YAML

                results[label] = run_single_model(model, system, prompt, fmt, use_json_schema, CRISIS_RUNS)

        log("\n" + "=" * 60)
        log(f" 結果サマリー (scenario=crisis, format={fmt})")
        log("=" * 60)
        log(f"  {'モデル × ペルソナ':<38} {'適合率':<8} {'平均(ms)':<10} {'avgTokens'}")
        log(f"  {'-'*38} {'-'*8} {'-'*10} {'-'*10}")
        for label, r in results.items():
            rate = f"{r['schema_ok']}/{r['runs']}"
            log(f"  {label:<38} {rate:<8} {r['avg_ms']:<10} {r['avg_tokens']}")

    log(f"\nログ保存先: {log_file}")


if __name__ == "__main__":
    main()
