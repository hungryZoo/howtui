#!/usr/bin/env bash
# 평가용 서버 컨테이너 (localhost만 바인딩). 로그는 logs/ 아래
set -u
ROOT=$(cd "$(dirname "$0")/../.." && pwd); M=$HOME/.cache/howtui-eval/models; TS=$(date +%Y%m%d-%H%M%S)
IMG_CPU=ghcr.io/ggml-org/llama.cpp:server-vulkan-b11429
run() { local name=$1; shift; docker rm -f "$name" >/dev/null 2>&1; docker run -d --name "$name" "$@" >/dev/null && (docker logs -f "$name" > "$ROOT/logs/${name}_$TS.log" 2>&1 &); }
run howtui-llama-cpu -p 127.0.0.1:8080:8080 -v "$M":/models:ro --entrypoint /app/llama-server $IMG_CPU -m /models/gemma-4-E2B_q4_0-it.gguf --host 0.0.0.0 --port 8080 -ngl 0 -c 8192 -t 8
run howtui-llama-cpu-e4b -p 127.0.0.1:18081:8080 -v "$M":/models:ro --entrypoint /app/llama-server $IMG_CPU -m /models/gemma-4-E4B_q4_0-it.gguf --host 0.0.0.0 --port 8080 -ngl 0 -c 8192 -t 8
docker ps --format '{{.Names}} {{.Status}}' | grep howtui-
