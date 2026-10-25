# 로컬 LLM 평가용 격리 실행 환경 (TC-LLM-021). 생성 명령어는 이 컨테이너 안에서만 실행 (--network none)
FROM mcr.microsoft.com/powershell:7.4-ubuntu-22.04
RUN apt-get update && apt-get install -y --no-install-recommends zsh tar gzip coreutils findutils grep sed gawk file && rm -rf /var/lib/apt/lists/*
COPY fixture.sh judge.sh /opt/eval/
RUN bash /opt/eval/fixture.sh /fixture && useradd -m runner
USER runner
