#!/usr/bin/env bash
# 고정 파일 구성 (평가 과제의 기대 출력 기준). 날짜는 고정값
set -eu
root=$1
mkdir -p "$root"/docs "$root"/src "$root"/photos "$root"/archive_src/a
cd "$root"
printf 'alpha foo\nbeta\ngamma foo\n' > docs/report.txt
printf 'foo notes\n' > docs/notes.txt
printf '# Title\nsome text\n' > docs/readme.md
for i in $(seq 1 30); do
  if [ $((i % 7)) -eq 0 ]; then echo "2026-10-01 ERROR job $i failed"; else echo "2026-10-01 INFO job $i ok"; fi
done > docs/old.log
printf 'print("main")\n' > src/main.py
printf 'def helper():\n    return 1\n' > src/util.py
printf 'console.log("app")\n' > src/app.js
head -c 5242880 /dev/zero > big.bin
head -c 2097152 /dev/zero > medium.bin
printf 'small\n' > small.txt
printf 'name,city,age\nkim,seoul,31\nlee,busan,28\npark,incheon,40\nchoi,daegu,35\njung,seoul,22\n' > data.csv
printf 'x' > photos/IMG_001.JPG; printf 'y' > photos/IMG_002.JPG; printf 'z' > photos/img_003.jpg
printf 'inside\n' > archive_src/a/b.txt
tar -czf backup.tar.gz -C archive_src a
rm -rf archive_src
printf 'secret\n' > .hidden
find . -exec touch -d '2026-10-01 12:00:00' {} +
chmod -R a+rX "$root"
