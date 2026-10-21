# 로컬 LLM 서버 연동 조사 (Phase 6)

- 조사일: 2026-10-22
- 대상: 이미 실행 중인 로컬 서버 연결 기능. (1) Ollama HTTP API, (2) llama.cpp `llama-server` HTTP API.
- 범위 제외: llama.cpp 내장, 모델 다운로드, 서버 실행과 관리.
- 근거 표기
  - 확인: 공식 문서 URL, 공식 저장소 소스 파일, 공식 API 응답의 직접 확인. 괄호 안 기호는 7절 출처 번호.
  - 미확인: 문서 기재 없음, 또는 실측 필요.
- 기준 버전
  - Ollama: 최신 릴리스 v0.40.7 (2026-10-18), 소스 기준 `main` 커밋 f580b54 (2026-10-21).
  - llama.cpp: 최신 빌드 릴리스 b11799 (2026-10-21), `tools/server/README.md` 최종 커밋 261700b (2026-10-21).
- 성능 수치 기재 없음. 측정 항목은 6절 벤치마크 계획으로 한정.

## 1. Ollama HTTP API

| 항목 | 내용 | 근거 |
| --- | --- | --- |
| 기본 주소 | `127.0.0.1:11434`, `OLLAMA_HOST` 환경 변수로 변경 | 확인 (O3) |
| 생존 확인 | `GET /` 또는 `HEAD /` 응답 본문 `Ollama is running` (텍스트) | 확인, 소스 (O10 routes.go:2369-2370) |
| 버전 | `GET /api/version` 응답 `{"version":"x.y.z"}`, `HEAD`도 등록 | 확인 (O1, O10 routes.go:2371-2372) |
| 채팅 | `POST /api/chat`. 필드: `model`(필수), `messages`(role: system, user, assistant, tool), `format`, `options`(temperature, num_ctx, num_predict, stop, seed 등), `stream`(기본 true), `think`(bool, 모델 정의 문자열, null), `keep_alive`(기본 `5m`), `tools`, `logprobs` | 확인 (O1, O2) |
| 스트리밍 형식 | NDJSON (`application/x-ndjson`), 한 줄당 JSON 객체 1개. 중간 객체 `message.content` 조각과 `done:false`. 최종 객체 `done:true`, `done_reason`, `total_duration`, `load_duration`, `prompt_eval_count`, `prompt_eval_duration`, `eval_count`, `eval_duration` (단위 ns). `stream:false` 시 단일 `application/json` | 확인 (O1, O2, O4) |
| 구조화 출력 | `format`에 문자열 `"json"` 또는 JSON 스키마 객체 지정. 지원 스키마 키워드 범위 문서 기재 없음 | 확인 (O1, O7), 지원 범위 미확인 |
| 모델 목록 | `GET /api/tags` 응답 `models[]`: `name`, `model`, `size`, `digest`, `details`(`format`, `family`, `parameter_size`, `quantization_level`) | 확인 (O1) |
| 모델 상세 | `POST /api/show` 응답 `capabilities`(예: `completion`, `vision`), `model_info`(아키텍처별 `context_length` 등), `template`, `parameters`. 모델별 `think` 수준 목록도 이 응답으로 확인 | 확인 (O1, O2, O6) |
| 적재 상태 | `GET /api/ps` 응답 `size`, `size_vram`, `expires_at` | 확인 (O1) |
| OpenAI 호환 | `POST /v1/chat/completions`, `POST /v1/completions`, `GET /v1/models`, `GET /v1/models/{model}`, `POST /v1/embeddings`, `POST /v1/responses`(비상태형만) | 확인 (O6) |
| OpenAI 호환 지원 필드 | 지원: `stream`, `stream_options.include_usage`, `response_format`, `seed`, `stop`, `temperature`, `top_p`, `max_tokens`, `tools`, `reasoning_effort`. 미지원: `tool_choice`, `logit_bias`, `n`, `user`, logprobs | 확인 (O6) |
| OpenAI 호환 JSON 스키마 | `response_format.type == "json_schema"` 시 `json_schema.schema`를 내부 `format`으로 전달 | 확인, 소스 (O11 openai.go:776-783) |
| OpenAI 호환 SSE | `data: {json}` 줄 반복, 종료 `data: [DONE]`. 사고 텍스트 필드명 `reasoning` | 확인, 소스 (O12 middleware/openai.go:124-159, O11 openai.go:36) |
| 사고 비활성 | OpenAI 호환 `reasoning_effort: "none"` 시 사고 비활성(false) 요청 | 확인 (O6) |
| 기본 컨텍스트 | FAQ 기준 4096 토큰. 컨텍스트 문서 기준 VRAM 24 GiB 미만 4k, 24-48 GiB 32k, 48 GiB 이상 256k. 변경: `OLLAMA_CONTEXT_LENGTH` 또는 `options.num_ctx`. OpenAI 호환 API에는 컨텍스트 지정 수단 없음 (Modelfile `num_ctx` 필요) | 확인 (O3, O6, O8) |
| 오류 형식 (네이티브) | HTTP 상태 코드(400, 404, 429, 500, 502) + 본문 `{"error":"..."}`. 스트림 도중 오류: NDJSON 줄 `{"error":"..."}`, 상태 코드 200 유지 | 확인 (O5) |
| 오류 형식 (OpenAI 호환) | `{"error":{"message","type","param","code"}}`, `type`: 400 `invalid_request_error`, 404 `not_found_error`, 그 외 `api_error` | 확인, 소스 (O11 openai.go:22-30, 249-260) |
| 취소 동작 | 문서 기재 없음. 소스상 `ChatHandler`가 HTTP 요청 context(`c.Request.Context()`)에서 파생한 context를 `Completion`에 전달. Go `net/http`는 클라이언트 연결 종료 시 요청 context 취소 (Go 문서). 연결 종료 시 실제 생성 중단 여부는 실측 필요 | 소스 확인 (O10 routes.go:3307, Go1), 종단 동작 미확인 |
| 인증 | 로컬 서버 인증 요구 기재 없음. 교차 출처 허용 기본값 `127.0.0.1`, `0.0.0.0`, 추가는 `OLLAMA_ORIGINS` | 확인 (O3) |

## 2. llama-server HTTP API

| 항목 | 내용 | 근거 |
| --- | --- | --- |
| 기본 주소 | `127.0.0.1:9931`. 2026-10-09 커밋 b42b7e6 "server: use port 9931 by default (#30159)" 이후 값. 이전 빌드 기본값 8080 (예고 커밋 0b14b87 "upcoming default port change 8080 --> 9931"). `--port`, `LLAMA_ARG_PORT`로 변경 | 확인 (L1, L2) |
| 상태 확인 | `GET /health`(`/v1/health` 동일). 200 `{"status":"ok"}`, 모델 적재 중 503 `{"error":{"code":503,"message":"Loading model","type":"unavailable_error"}}`. API 키 검사 제외 | 확인 (L1, L6 server.cpp:280) |
| OpenAI 호환 채팅 | `POST /v1/chat/completions`. README 문구: "no strong claims of compatibility with OpenAI API spec". `/completion` 계열 확장 필드(mirostat 등) 함께 허용 | 확인 (L1) |
| 비 OpenAI 생성 | `POST /completion`. `prompt` 문자열 또는 토큰 배열, `stream`, `grammar`, `json_schema`, `n_predict` 등. 스트림 응답 필드 `content`, `tokens`, `stop`. 종료 표식 `[DONE]` 미송신 | 확인 (L1, L4 server-context.cpp:5365-5376) |
| 모델 정보 | `GET /v1/models`. 단일 모델 모드: 요소 1개, `id` 기본값은 `-m` 경로(`--alias`로 변경), `meta`(`n_ctx_train`, `n_params`, `size`), 적재 중 `meta` null 가능. 라우터 모드(모델 미지정 실행): `/v1/models`와 `/models` 모두 라우터 목록 응답, `status.value`(예: `loaded`) 포함 | 확인 (L1, L6 server.cpp:271, 285-286) |
| 서버 속성 | `GET /props`: `default_generation_settings.n_ctx`, `total_slots`, `model_path`, `chat_template`, `chat_template_caps`, `modalities`, `build_info`, `is_sleeping`. 라우터 모드는 `?model=` 지정 | 확인 (L1) |
| 스트리밍 형식 | SSE (`text/event-stream`). `data: {json}` + 빈 줄. OpenAI 호환 경로 종료 `data: [DONE]`. 무출력 구간 핑 주석 `:` 줄 (기본 30초, `--sse-ping-interval`). 스트림 도중 오류 `data: {"error":{...}}` 후 종료, `[DONE]` 없음 | 확인, 소스 (L1, L4 server-context.cpp:5335-5420) |
| 구조화 출력 | `response_format`: `{"type":"json_object"}`(스키마 없음은 임의 객체), `{"type":"json_object","schema":{...}}`, `{"type":"json_schema","json_schema":{"schema":{...}}}`(OpenAI 형식). 확장 필드 `json_schema`, `grammar`(GBNF), 두 필드 동시 지정 시 오류. 그 외 `type` 값은 오류 | 확인, 소스 (L1, L3 server-common.cpp:1373-1398) |
| 구조화 출력 주의 | README 예시는 `{"type":"json_schema","schema":{...}}` 형태 표기. 소스는 `type == "json_schema"` 시 `response_format.json_schema.schema`만 읽음. README 형태 사용 시 스키마 미적용(빈 스키마, 임의 객체) 경로 | 소스 확인 (L3 server-common.cpp:1387-1398), 실측 미확인 |
| 스키마 지원 범위 | JSON 스키마를 문법으로 변환. 지원 기능은 `tests/test-json-schema-to-grammar.cpp` 참조 (README) | 확인 (L1), 상세 범위 미확인 |
| 사고 출력 | `--reasoning-format`(기본 auto), `deepseek` 형식 시 `message.reasoning_content`. 요청 필드 `reasoning_effort: "none"` 시 사고 비활성, `chat_template_kwargs`(예: `{"enable_thinking": false}`) 전달 가능 | 확인 (L1, L7 chat.cpp:203) |
| 취소 동작 | 문서 기재 없음. 소스상 HTTP 계층 `should_stop`이 연결 종료(`is_connection_closed`, 소켓 닫힘)와 연결, 응답 대기 중 `should_stop` 참이면 `server_response_reader::stop()`이 미완료 작업에 `SERVER_TASK_TYPE_CANCEL` 전송 | 소스 확인 (L5 server-http.cpp:686-808, server-queue.cpp:549-612), 실측 미확인 |
| 실행 중 제어 | `POST /v1/chat/completions/control`: 현재 `action: "reasoning_end"`만 지원 (사고 블록 조기 종료). 생성 취소 기능 아님 | 확인 (L1) |
| 오류 형식 | OpenAI 형식 `{"error":{"code","message","type"}}`. 추가 type: `not_supported_error`(501), `unavailable_error`(503). 잘못된 grammar: 400 `invalid_request_error` "Failed to parse grammar" | 확인 (L1) |
| 인증 | `--api-key` 지정 시 API 키 필요 (`Authorization: Bearer` 헤더, README 예시 기준). `/health`는 공개 | 확인 (L1) |
| 타임아웃 | `--timeout` 읽기와 쓰기 기본 3600초 | 확인 (L1) |
| 컨텍스트 | `-c, --ctx-size` 기본 0 (모델 메타데이터 값 사용). 서버 시작 시 결정, 요청 단위 변경 수단 기재 없음 | 확인 (L1) |

### 2.1 Gemma 채팅 템플릿 처리

- `--jinja` 기본 활성, 채팅 템플릿 기본값은 모델 메타데이터 템플릿. 확인 (L1)
- Google QAT GGUF 메타데이터 `chat_template` 첫 주석: "Google Gemma 4 Canonical Chat Template", "Published: 2026-07-09". 확인 (G2, G3 HF API `gguf.chat_template`)
- llama.cpp `common/chat.cpp`에 Gemma 4 전용 형식 감지(`COMMON_CHAT_FORMAT_PEG_GEMMA4`) 존재, 구형 Gemma 4 템플릿 감지 시 경고와 호환 처리. 확인 (L7 chat.cpp:837, 1083-1087, 1219-1227)
- 내장 템플릿 목록에 `gemma` 포함 (`--chat-template` 설명). 확인 (L1)
- Gemma 4 모델 카드 문구: "many libraries like Transformers and llama.cpp handle the complexities of the chat template for you". 확인 (G2)
- howtui 측 결론: 두 서버 모두 OpenAI 형식 `messages`만 전달, 템플릿 직접 조립 불필요. 사고 토큰(`<|think|>`) 수동 삽입 대신 `reasoning_effort` 사용.

## 3. Gemma 4 E2B, E4B 공식 정보

### 3.1 모델 사양

| 항목 | E2B | E4B | 근거 |
| --- | --- | --- | --- |
| 파라미터 (모델 카드) | 2.3B effective (임베딩 포함 5.1B) | 4.5B effective (임베딩 포함 8B) | 확인 (G2, G3) |
| HF safetensors 총계 (`google/gemma-4-*-it`) | 5,123,178,051 | 7,996,156,490 | 확인 (G5 API) |
| QAT GGUF 메타데이터 파라미터 수 (텍스트 모델 파일) | 4,628,569,635 | 7,463,013,674 | 확인 (G2, G3 API `gguf.total`) |
| 레이어 | 35 | 42 | 확인 (G2) |
| 슬라이딩 윈도 | 512 토큰 | 512 토큰 | 확인 (G2) |
| 컨텍스트 길이 | 128K (GGUF `context_length` 131072) | 128K (131072) | 확인 (G1, G2, G3) |
| 어휘 크기 | 262K | 262K | 확인 (G2) |
| 입력 모달리티 | 텍스트, 이미지, 오디오 | 텍스트, 이미지, 오디오 | 확인 (G2) |
| 비전, 오디오 인코더 | 약 150M, 약 300M | 약 150M, 약 300M | 확인 (G2) |
| 라이선스 | Apache 2.0 | Apache 2.0 | 확인 (G2, G3 카드 `license: apache-2.0`, G4 본문 "Apache License Version 2.0") |
| gated (라이선스 동의 필요 여부) | `gated: false` | `gated: false` | 확인 (G2, G3, G5 API). QAT GGUF와 원본 `-it` 저장소 모두 false |
| 권장 샘플링 | temperature 1.0, top_p 0.95, top_k 64 | 동일 | 확인 (G2) |

- 사고 모드: 시스템 프롬프트 시작의 `<|think|>` 토큰으로 활성. 출력 구조 `<|channel>thought\n...<channel|>`. E2B와 E4B는 사고 비활성 시 빈 사고 블록 미생성 (카드 문구 "For all models except for the E2B and E4B variants"). 다중 턴 이력에는 이전 사고 내용 제외 권장. 확인 (G2)
- 공식 문서 페이지 상단 문구: "This documentation is deprecated. Please visit the new documentation at gemma.ai." 확인 (G1)

### 3.2 Google QAT Q4_0 GGUF 파일 (HF tree API)

| 저장소 | 텍스트 모델 파일 | 크기 (바이트) | 비전 투영 파일 (mmproj) | 크기 (바이트) |
| --- | --- | --- | --- | --- |
| `google/gemma-4-E2B-it-qat-q4_0-gguf` | `gemma-4-E2B_q4_0-it.gguf` | 3,349,516,256 (3.35 GB) | `gemma-4-E2B-it-mmproj.gguf` | 986,833,664 (0.99 GB) |
| `google/gemma-4-E4B-it-qat-q4_0-gguf` | `gemma-4-E4B_q4_0-it.gguf` | 5,154,941,280 (5.15 GB) | `gemma-4-E4B-it-mmproj.gguf` | 991,552,256 (0.99 GB) |

- GB 표기는 10^9 바이트 기준. 확인 (G2, G3 tree API)
- howtui 용도(텍스트 전용)에서 mmproj 파일 필요 여부: llama-server는 mmproj 미지정 실행 가능 여부 실측 필요. 미확인

### 3.3 공식 메모리 요구량 (가중치 적재 기준)

| 모델 | BF16 | SFP8 | Q4_0 | Mobile | Mobile (텍스트 전용) |
| --- | --- | --- | --- | --- | --- |
| Gemma 4 E2B | 11.4 GB | 5.7 GB | 2.9 GB | 1.1 GB | 0.84 GB |
| Gemma 4 E4B | 17.9 GB | 8.9 GB | 4.5 GB | 2.5 GB | 2.2 GB |

- 문서 표 설명: 파라미터 수, 양자화 수준, 추가 적재분 20% 오버헤드 기준의 GPU 또는 TPU 메모리 근사치. Mobile 수치는 LiteRT-LM 기준. 확인 (G1)
- 지원 소프트웨어와 컨텍스트 창(KV 캐시) 메모리 제외. 확인 (G1)
- 표의 Q4_0 수치(E2B 2.9 GB)와 QAT GGUF 파일 크기(3.35 GB) 차이 원인: 문서 기재 없음. 미확인

### 3.4 공식 문서 기재 런타임

- 로컬 실행: LM Studio, Ollama. 엣지 실행: LiteRT-LM, llama.cpp, MLX. Python: Hugging Face Transformers, Keras, Unsloth, Tunix. QAT 안내 표: vLLM, SGLang, llama.cpp, LM Studio. 모델 크기별 구분 없이 기재. 확인 (G1)

### 3.5 Ollama 레지스트리 태그 (매니페스트 메타데이터)

| 태그 | 모델 레이어 (바이트) | projector (바이트) | 매니페스트 합계 |
| --- | --- | --- | --- |
| `gemma4:e2b-it-qat` | 3,349,514,112 | 986,833,312 | 4.34 GB |
| `gemma4:e4b-it-qat` | 5,154,939,136 | 991,551,904 | 6.15 GB |
| `gemma4:e2b` | 3,502,493,472 | 986,833,664 | 4.59 GB (draft 레이어 포함) |
| `gemma4:e4b` | 5,493,439,296 | 991,552,256 | 6.58 GB (draft 레이어 포함) |

- `gemma4:latest`는 `gemma4:e4b`와 동일 digest. `q4_0` 접미사 태그 없음, QAT 변형은 `-it-qat` 태그. 컨텍스트 128K 표기. 확인 (O15)
- `-it-qat` 모델 레이어와 Google QAT GGUF 파일 크기 차이 2,144 바이트. 동일 가중치 여부 미확인

### 3.6 Unsloth GGUF

- 저장소: `unsloth/gemma-4-E2B-it-GGUF`, `unsloth/gemma-4-E4B-it-GGUF`(기반 `google/gemma-4-E*-it`), `unsloth/gemma-4-E2B-it-qat-GGUF`, `unsloth/gemma-4-E4B-it-qat-GGUF`(기반 `google/gemma-4-E*-it-qat-q4_0-unquantized`). 전체 `gated: false`, `license: apache-2.0`, GGUF 컨텍스트 131072. 확인 (U1-U4 API)
- `UD-` 접두: Unsloth Dynamic 2.0 GGUF (README 링크 기준). 확인 (U1)

| 양자화 | E2B (GB) | E4B (GB) |
| --- | --- | --- |
| UD-IQ2_M | 2.29 | 3.55 |
| UD-IQ3_XXS | 2.37 | 3.72 |
| UD-Q2_K_XL | 2.40 | 3.76 |
| Q3_K_S | 2.45 | 3.86 |
| Q3_K_M | 2.54 | 4.06 |
| UD-Q3_K_XL | 2.92 | 4.59 |
| IQ4_XS | 2.98 | 4.72 |
| IQ4_NL | 3.04 | 4.84 |
| Q4_0 | 3.04 | 4.84 |
| Q4_K_S | 3.04 | 4.84 |
| Q4_K_M | 3.11 | 4.98 |
| Q4_1 | 3.15 | 5.07 |
| UD-Q4_K_XL | 3.18 | 5.13 |
| Q5_K_S | 3.32 | 5.40 |
| Q5_K_M | 3.36 | 5.48 |
| UD-Q5_K_XL | 4.29 | 6.66 |
| Q6_K | 4.50 | 7.07 |
| UD-Q6_K_XL | 4.71 | 7.46 |
| Q8_0 | 5.05 | 8.19 |
| UD-Q8_K_XL | 5.28 | 8.71 |
| BF16 | 9.31 | 15.05 |
| QAT 저장소 UD-Q2_K_XL | 2.19 | 3.22 |
| QAT 저장소 UD-Q4_K_XL | 2.62 | 4.22 |

- 부가 파일: mmproj BF16 0.99 GB, F16 0.99 GB, F32 1.90-1.91 GB. MTP 드래프터 `mtp-gemma-4-E*-it.gguf` 0.10 GB(일반 저장소), 0.06 GB(QAT 저장소). 확인 (U1-U4 tree API)
- 품질과 속도 비교: 조사 범위 외. 6절 측정 대상

## 4. 공통 provider 인터페이스 시사점

### 4.1 공통 경로 (확인 사항 기반)

- 두 서버 모두 `POST /v1/chat/completions` 제공, SSE `data:` 줄 + 종료 `data: [DONE]`. (O6, O12, L1, L4)
- 공통 요청 필드: `model`, `messages`, `stream`, `temperature`, `top_p`, `max_tokens`, `stop`, `response_format`, `reasoning_effort`. (O6, L1, L3 server-common.cpp:1366-1371). `seed`의 llama-server OpenAI 경로 처리 여부는 미확인.
- 구조화 출력: 두 서버 모두 OpenAI 형식 `{"type":"json_schema","json_schema":{"schema":{...}}}` 처리. (O11, L3)
- 모델 목록: 두 서버 모두 `GET /v1/models`. (O6, L1)
- 오류 본문: 두 서버 모두 `error.message`, `error.type` 필드. (O11, L1)
- 결론: 채팅, 스트리밍, 구조화 출력은 OpenAI 호환 어댑터 1개로 처리 가능.

### 4.2 서버별 잔여 차이

| 구분 | Ollama | llama-server | 어댑터 처리 |
| --- | --- | --- | --- |
| 기본 포트 | 11434 | 9931 (구 빌드 8080) | 탐색 후보 3개 |
| 서버 식별 | `GET /api/version` | `GET /health`, `GET /props`(`build_info`) | `probe()`에서 순차 시도 |
| 준비 상태 | 별도 준비 상태 응답 기재 없음, 빈 `messages`의 `/api/chat` 요청으로 사전 적재 가능 (O1) | 적재 중 `/health` 503 | 503 시 재시도 상태 표시 |
| 모델 메타데이터 | `/api/tags`(`parameter_size`, `quantization_level`), `/api/show`(`capabilities`, `context_length`) | `/v1/models` `meta.n_ctx_train`, `/props` `n_ctx` | flavor별 보강 조회 |
| 실효 컨텍스트 | 기본 4096 (VRAM 24 GiB 미만), OpenAI 경로에서 변경 불가, 네이티브 `options.num_ctx` 가능 | 서버 시작 시 `-c`로 고정, `/props`로 조회 | `context_window()` 조회 후 프롬프트 예산 산정 |
| 사고 텍스트 필드 | `delta.reasoning` | `delta.reasoning_content` | 두 필드 모두 파싱, 기본 `reasoning_effort: "none"` |
| SSE 주석 핑 | 기재 없음 | `:` 줄 송신 | `:` 시작 줄 무시 |
| 스트림 도중 오류 | OpenAI 경로 형식 미확인 (네이티브는 `{"error":"..."}` 줄) | `data: {"error":{...}}` | `error` 키 존재 시 오류 이벤트 |
| 인증 | 로컬 기재 없음 | `--api-key` 사용 시 Bearer | 선택적 API 키 설정 |
| 스키마 지원 범위 | 문서 기재 없음 | 문법 변환 테스트 기준 | 단순 스키마 한정 + 클라이언트 측 검증 |
| 취소 | 요청 context 전달 (소스) | 연결 종료 시 CANCEL 작업 (소스) | 스트림 drop으로 연결 종료, 실측 검증 |

### 4.3 권장 어댑터 설계

- 단일 `OpenAiCompatProvider` + `ServerFlavor { Ollama, LlamaServer, Generic }` 구조. 채팅 경로는 공통, 차이는 flavor 훅 4개(식별, 모델 목록 보강, 컨텍스트 조회, 사고 필드명)로 한정.
- 인터페이스 초안 (Rust, 설계용)

```rust
pub trait LlmProvider: Send + Sync {
    async fn probe(&self) -> Result<ServerInfo>;            // flavor, version, ready
    async fn list_models(&self) -> Result<Vec<ModelInfo>>;  // /v1/models + flavor 보강
    async fn context_window(&self, model: &str) -> Result<Option<u32>>;
    async fn chat_stream(
        &self,
        req: ChatRequest,                 // messages, json_schema, sampling, reasoning_effort
        cancel: CancellationToken,        // 취소 시 응답 스트림 drop
    ) -> Result<BoxStream<'static, Result<ChatEvent>>>;
}

pub enum ChatEvent {
    Content(String),
    Reasoning(String),                    // reasoning 또는 reasoning_content
    Done { finish_reason: Option<String>, usage: Option<Usage> },
}
```

- 구조화 출력: `response_format`은 OpenAI 형식 중첩(`json_schema.schema`)만 사용. README 단축 형태(`{"type":"json_schema","schema":...}`) 사용 금지.
- 응답 검증: 서버 측 문법 제약 여부와 별개로 클라이언트 측 JSON 스키마 검증과 재시도 1회.
- Ollama 컨텍스트 4096 제약: Phase 6 초기 범위는 OpenAI 경로 고정, 시스템 프롬프트와 이력 예산을 `context_window()` 기준으로 절단. 부족 시 Ollama 네이티브 `/api/chat`(`options.num_ctx`) 경로를 후속 옵션으로 유지.
- 서버 관리 기능 제외 원칙 유지: 모델 pull, 적재, 해제 API(`/api/pull`, `/models/load` 등) 미호출.

## 5. Raspberry Pi 관련 사실

| 항목 | 내용 | 근거 |
| --- | --- | --- |
| Ollama linux arm64 | 공식 릴리스 자산 `ollama-linux-arm64.tar.zst`(+ jetpack5, jetpack6 변형), Linux 문서 "ARM64 install" 절 | 확인 (O9, O14) |
| Ollama armv7 (32-bit) | v0.40.7 릴리스 자산에 armv7 없음. `install.sh` 아키텍처 분기: `x86_64`는 amd64, `aarch64` 또는 `arm64`는 arm64, 그 외 "Unsupported architecture" 오류 | 확인 (O13 install.sh:35-39, O14) |
| Ollama Raspberry Pi 명시 | 문서 내 Raspberry Pi 명시 지원 문구 | 미확인 (기재 없음) |
| llama.cpp arm64 | 공식 빌드 자산 `llama-b11799-bin-ubuntu-arm64.tar.gz`, `ubuntu-vulkan-arm64` 등. 빌드 문서 KleidiAI 표에 Linux AArch64 기재 | 확인 (L8, L10) |
| llama.cpp armv7 바이너리 | 릴리스 자산에 armv7 없음 | 확인 (L10) |
| llama.cpp armv7 소스 빌드 | CMake 아키텍처 판정 정규식 `^(aarch64\|arm.*\|ARM64)$`(armv7l 일치), `ggml-cpu-impl.h`에 "32-bit ARM compatibility" 코드 구역 존재. KleidiAI는 32-bit Arm 비허용. armv7 공식 지원 명시 문구 없음 | 소스 확인 (L9), 빌드 성공과 동작 미확인 |
| 32-bit 주소 공간 | 32-bit 프로세스 가상 주소 공간 최대 4 GiB. E2B QAT GGUF(3.35 GB) 적재 가능 여부 | 적재 가능 여부 미확인 (실측 필요) |

## 6. 벤치마크 계획 (결과 없음)

- 공통 조건: 모델(E2B QAT Q4_0, E4B QAT Q4_0, Unsloth UD-Q4_K_XL 등), 서버(Ollama, llama-server), 샘플링(카드 권장값, temperature 0), 사고 모드(on, off), 구조화 출력(on, off), 반복 3회, 하드웨어와 서버 버전 기록.
- 과제 세트: 셸별 자연어 요청과 판정 기준(기대 출력 또는 판정 스크립트) 묶음. 위험 요청 세트와 정상 요청 세트 분리.

| 지표 | 정의 | 측정 방법 |
| --- | --- | --- |
| Bash 명령 정확도 | 판정 통과 과제 비율 | 생성 명령을 격리 컨테이너(고정 파일 시스템 픽스처)에서 bash 실행, 기대 출력 또는 판정 스크립트 비교 |
| Zsh 명령 정확도 | 판정 통과 과제 비율 | 동일 절차, zsh 컨테이너, zsh 전용 문법(글로브 한정자 등) 과제 포함 |
| PowerShell 명령 정확도 | 판정 통과 과제 비율 | 동일 절차, pwsh 7 컨테이너 또는 Windows VM, cmdlet과 파이프라인 객체 출력 비교 |
| 요구사항 충족률 | 과제별 요구 조건 충족 비율 | 과제마다 조건 체크리스트(대상 경로, 필터, 정렬, 출력 형식) 작성, 자동 판정 + 무작위 20% 수동 검수 |
| 오답 명령 비율 | 실행 불가 또는 잘못된 명령 비율 | 구문 검사(`bash -n`, `zsh -n`, PowerShell 파서) 실패, 존재하지 않는 명령이나 옵션, 비정상 종료 코드 집계 |
| 위험 명령 처리 | 위험 요청 대상 경고, 거부, 안전 대안 제시 비율 | 위험 요청 세트(재귀 삭제, 디스크 덮어쓰기, 권한 일괄 변경 등) 실행 없이 응답 분류, 정상 요청 세트 대상 오탐 비율 병기, howtui 자체 위험 판정 결과와 분리 기록 |
| 메모리 사용량 | 서버 프로세스 최대 상주 메모리 | `/proc/<pid>/status`의 `VmHWM`, Ollama `/api/ps`의 `size`, `size_vram`, 컨텍스트 4096과 8192 각각 기록 |
| 첫 토큰 시간 | 요청 송신부터 첫 content 조각 수신까지 시간 | howtui 클라이언트 측 단조 시계 측정, 콜드(모델 미적재)와 웜 분리, Ollama `load_duration`, `prompt_eval_duration` 병기 |
| 초당 토큰 | 생성 토큰 수 / 생성 시간 | Ollama `eval_count / eval_duration`, llama-server 응답 `timings.predicted_per_second`(L1), 클라이언트 측 측정값 병기 |
| CPU 전용 성능 | GPU 미사용 조건의 위 3개 지표 | GPU 없는 장비(x86_64 노트북, arm64 Raspberry Pi) 또는 llama-server `--n-gpu-layers 0`, 스레드 수와 CPU 모델 기록 |
| 후속 수정 성공률 | 수정 요청 후 명령의 신규 조건 충족 + 기존 조건 유지 비율 | 1차 명령 후 수정 요청(조건 추가, 셸 변경 등) 1-3턴, 턴별 판정 |

## 7. 출처

- Ollama
  - (O1) https://github.com/ollama/ollama/blob/main/docs/api.md
  - (O2) https://docs.ollama.com/api/chat
  - (O3) https://github.com/ollama/ollama/blob/main/docs/faq.mdx
  - (O4) https://github.com/ollama/ollama/blob/main/docs/api/streaming.mdx
  - (O5) https://github.com/ollama/ollama/blob/main/docs/api/errors.mdx
  - (O6) https://github.com/ollama/ollama/blob/main/docs/api/openai-compatibility.mdx
  - (O7) https://github.com/ollama/ollama/blob/main/docs/capabilities/structured-outputs.mdx
  - (O8) https://github.com/ollama/ollama/blob/main/docs/context-length.mdx
  - (O9) https://github.com/ollama/ollama/blob/main/docs/linux.mdx
  - (O10) https://github.com/ollama/ollama/blob/main/server/routes.go (커밋 f580b54)
  - (O11) https://github.com/ollama/ollama/blob/main/openai/openai.go
  - (O12) https://github.com/ollama/ollama/blob/main/middleware/openai.go
  - (O13) https://github.com/ollama/ollama/blob/main/scripts/install.sh
  - (O14) https://github.com/ollama/ollama/releases/tag/v0.40.7
  - (O15) https://ollama.com/library/gemma4/tags, https://registry.ollama.ai/v2/library/gemma4/manifests/e2b-it-qat (e4b-it-qat, e2b, e4b 동일 경로)
- llama.cpp
  - (L1) https://github.com/ggml-org/llama.cpp/blob/master/tools/server/README.md
  - (L2) https://github.com/ggml-org/llama.cpp/blob/master/common/common.h (630행 `port = 9931`), 커밋 b42b7e6 (#30159), 커밋 0b14b87 (#26508)
  - (L3) https://github.com/ggml-org/llama.cpp/blob/master/tools/server/server-common.cpp
  - (L4) https://github.com/ggml-org/llama.cpp/blob/master/tools/server/server-context.cpp
  - (L5) https://github.com/ggml-org/llama.cpp/blob/master/tools/server/server-http.cpp, https://github.com/ggml-org/llama.cpp/blob/master/tools/server/server-queue.cpp
  - (L6) https://github.com/ggml-org/llama.cpp/blob/master/tools/server/server.cpp
  - (L7) https://github.com/ggml-org/llama.cpp/blob/master/common/chat.cpp
  - (L8) https://github.com/ggml-org/llama.cpp/blob/master/docs/build.md
  - (L9) https://github.com/ggml-org/llama.cpp/blob/master/ggml/cmake/common.cmake, https://github.com/ggml-org/llama.cpp/blob/master/ggml/src/ggml-cpu/ggml-cpu-impl.h
  - (L10) https://github.com/ggml-org/llama.cpp/releases/tag/b11799
- Gemma
  - (G1) https://ai.google.dev/gemma/docs/core
  - (G2) https://huggingface.co/google/gemma-4-E2B-it-qat-q4_0-gguf, https://huggingface.co/api/models/google/gemma-4-E2B-it-qat-q4_0-gguf, 동일 경로 `/tree/main`
  - (G3) https://huggingface.co/google/gemma-4-E4B-it-qat-q4_0-gguf, https://huggingface.co/api/models/google/gemma-4-E4B-it-qat-q4_0-gguf, 동일 경로 `/tree/main`
  - (G4) https://ai.google.dev/gemma/docs/gemma_4_license
  - (G5) https://huggingface.co/api/models/google/gemma-4-E2B-it, https://huggingface.co/api/models/google/gemma-4-E4B-it
- Unsloth
  - (U1) https://huggingface.co/unsloth/gemma-4-E2B-it-GGUF
  - (U2) https://huggingface.co/unsloth/gemma-4-E4B-it-GGUF
  - (U3) https://huggingface.co/unsloth/gemma-4-E2B-it-qat-GGUF
  - (U4) https://huggingface.co/unsloth/gemma-4-E4B-it-qat-GGUF
- 기타
  - (Go1) https://pkg.go.dev/net/http#Request.Context
