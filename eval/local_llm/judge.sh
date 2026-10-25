#!/usr/bin/env bash
# 표준 입력의 명령어를 고정 파일 복사본에서 실행하고 출력과 사후 점검 결과를 출력
# 사용: judge.sh <bash|zsh|pwsh> <post 명령 (bash)>
shell=$1; post=$2
cmd=$(cat)
work=$(mktemp -d /tmp/w.XXXX); cp -a /fixture/. "$work"/; cd "$work"
printf '%s\n' "$cmd" > /tmp/cmd.$$
case $shell in
  bash) timeout 15 bash /tmp/cmd.$$ </dev/null 2>/tmp/err.$$; rc=$? ;;
  zsh) timeout 15 zsh -f /tmp/cmd.$$ </dev/null 2>/tmp/err.$$; rc=$? ;;
  pwsh) cp /tmp/cmd.$$ /tmp/cmd.$$.ps1; timeout 30 pwsh -NoProfile -NonInteractive -File /tmp/cmd.$$.ps1 </dev/null 2>/tmp/err.$$; rc=$? ;;
esac
printf '\n@@RC %s\n@@POST\n' "$rc"
bash -c "$post" 2>&1
printf '@@SYNTAX\n'
case $shell in
  bash) bash -n /tmp/cmd.$$ 2>&1 && echo ok ;;
  zsh) zsh -n /tmp/cmd.$$ 2>&1 && echo ok ;;
  pwsh) pwsh -NoProfile -Command "\$e=\$null; [void][System.Management.Automation.Language.Parser]::ParseFile('/tmp/cmd.$$.ps1',[ref]\$null,[ref]\$e); if (\$e.Count) { \$e | % { \$_.Message } } else { 'ok' }" 2>&1 ;;
esac
printf '@@STDERR\n'; head -c 2000 /tmp/err.$$
