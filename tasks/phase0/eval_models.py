#!/usr/bin/env python3
"""Phase 0: ローカルLLMモデル比較評価 (第2ラウンド — 最新モデル)

Run: python3 tasks/phase0/eval_models.py
Logs: tasks/phase0/logs/eval_YYYYMMDD_HHMMSS.log
"""

from __future__ import annotations

import json
import os
import re
import sys
import time
import urllib.request
import urllib.error
from datetime import datetime
from pathlib import Path
from typing import Optional

OLLAMA_URL = "http://localhost:11434"

SYSTEM = "You are a JSON-only responder. All fields must be non-null Japanese strings. No explanation, no markdown."

PROMPT = """\
あなたは石器時代の小さな集落に住む村人「タカシ」です。性格: 好奇心旺盛。今は空腹です。
隣の村人「ユキ」と焚き火の前で会話しています。道具は石斧と槍しかありません。

以下の4フィールドを持つJSONで応答してください。全フィールド必須、nullや空文字は禁止です。
- speech: タカシがユキに話すセリフ
- inner_thought: タカシの内心（口には出さない）
- action: タカシの具体的な身体動作（例: 立ち上がる、石斧を手に取る、火をかき混ぜる）
- emotion_change: 感情の変化（例: 不安が募る、期待で胸が高鳴る）

例文とは異なる独自の内容で応答してください。"""

# Ollama structured output: constrained decoding で string型を強制
RESPONSE_SCHEMA = {
    "type": "object",
    "properties": {
        "speech": {"type": "string"},
        "inner_thought": {"type": "string"},
        "action": {"type": "string"},
        "emotion_change": {"type": "string"},
    },
    "required": ["speech", "inner_thought", "action", "emotion_change"],
}

EXPECTED_KEYS = {"speech", "inner_thought", "action", "emotion_change"}

MODELS = ["qwen3.5:2b", "qwen3.5:0.8b", "gemma4:e2b"]
RUNS = 5

# --- logging ---
log_dir = Path(__file__).parent / "logs"
log_dir.mkdir(exist_ok=True)
log_file = log_dir / f"eval_{datetime.now():%Y%m%d_%H%M%S}.log"

def log(msg: str):
    print(msg)
    with open(log_file, "a") as f:
        f.write(msg + "\n")

def debug(msg: str):
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

def pull_model(model: str):
    log(f"  {model} pulling...")
    os.system(f"ollama pull {model}")

def generate(model: str, system: str, prompt: str) -> dict:
    """Use /api/chat with JSON Schema structured output."""
    body = {
        "model": model,
        "messages": [
            {"role": "system", "content": system},
            {"role": "user", "content": prompt},
        ],
        "stream": False,
        "format": RESPONSE_SCHEMA,
        "options": {
            "num_predict": 256,
        },
    }
    # Qwen: disable thinking mode
    if "qwen" in model:
        body["think"] = False
    return ollama_request("/api/chat", body)

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

# --- main ---
def main():
    if not check_ollama():
        log("ERROR: Ollama が起動していません。 'ollama serve' を実行してください。")
        sys.exit(1)

    log("モデル確認中...")
    installed = list_models()
    debug(f"installed models: {installed}")
    for model in MODELS:
        if model in installed:
            log(f"  {model} OK")
        else:
            pull_model(model)

    log("")
    log("=" * 42)
    log(f" 評価開始 (各モデル {RUNS}回)")
    log("=" * 42)

    results = {}

    for model in MODELS:
        log(f"\n===== {model} =====")
        schema_ok = 0
        times = []

        for i in range(1, RUNS + 1):
            try:
                start = time.monotonic()
                resp = generate(model, SYSTEM, PROMPT)
                elapsed_ms = int((time.monotonic() - start) * 1000)
            except Exception as e:
                log(f"  [{i}/{RUNS}] ERROR: {e}")
                debug(f"exception type={type(e).__name__}")
                continue

            if "error" in resp:
                log(f"  [{i}/{RUNS}] ERROR: {resp['error']}")
                continue

            debug(f"full response keys: {list(resp.keys())}")
            debug(f"full response dump: {json.dumps(resp, ensure_ascii=False)[:800]}")
            msg = resp.get("message", {})
            content = msg.get("content", "")
            debug(f"raw content ({len(content)} chars): {repr(content[:500])}")
            debug(f"total_duration={resp.get('total_duration', '?')} eval_count={resp.get('eval_count', '?')}")

            if not content.strip():
                log(f"  [{i}/{RUNS}] ERROR: content が空")
                continue

            times.append(elapsed_ms)

            # Strip <think>...</think> block if present
            clean = content
            think_match = re.search(r"<think>.*?</think>\s*", clean, re.DOTALL)
            if think_match:
                debug(f"stripped thinking block ({len(think_match.group())} chars)")
                clean = clean[think_match.end():]

            try:
                parsed = json.loads(clean)
            except json.JSONDecodeError as e:
                log(f"  [{i}/{RUNS}] {elapsed_ms}ms ✗ JSON parse failed: {e}")
                log(f"  {clean[:300]}")
                continue

            issues = validate_response(parsed)
            if not issues:
                schema_ok += 1
                log(f"  [{i}/{RUNS}] {elapsed_ms}ms ✓ schema OK")
            else:
                log(f"  [{i}/{RUNS}] {elapsed_ms}ms ✗ schema NG: {', '.join(issues)}")

            log(f"  {json.dumps(parsed, ensure_ascii=False, indent=2)}")

        avg = int(sum(times) / len(times)) if times else 0
        log(f"  --- スキーマ適合率: {schema_ok}/{RUNS} | 平均応答: {avg}ms ---")
        results[model] = {"schema_ok": schema_ok, "avg_ms": avg, "runs": RUNS}

    # --- summary ---
    log("\n" + "=" * 42)
    log(" 結果サマリー")
    log("=" * 42)
    log(f"  {'モデル':<20} {'適合率':<10} {'平均(ms)':<10} {'3秒以内':<8}")
    log(f"  {'-'*20} {'-'*10} {'-'*10} {'-'*8}")
    for model, r in results.items():
        rate = f"{r['schema_ok']}/{r['runs']}"
        under_3s = "✓" if r["avg_ms"] <= 3000 else "✗"
        log(f"  {model:<20} {rate:<10} {r['avg_ms']:<10} {under_3s:<8}")

    log(f"\nログ保存先: {log_file}")

if __name__ == "__main__":
    main()
