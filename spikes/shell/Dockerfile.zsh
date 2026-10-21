# SPIKE-04 실험용 zsh 이미지
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends zsh locales && rm -rf /var/lib/apt/lists/* \
 && sed -i 's/# ko_KR.UTF-8/ko_KR.UTF-8/; s/# C.UTF-8/C.UTF-8/' /etc/locale.gen; locale-gen >/dev/null 2>&1 || true
ENV LANG=C.UTF-8
