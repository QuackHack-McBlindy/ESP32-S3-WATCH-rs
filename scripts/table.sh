#!/usr/bin/env bash
set -euo pipefail

source_readme="/home/pungkula/dotfiles/README.md"
target_readme="./README.md"


regex_patterns=$(grep -oP '\d+(?= dynamically generated regex patterns)' "$source_readme" || echo "unknown")
voice_scripts=$(grep -oP '\d+(?= scripts have voice commands)' "$source_readme" || echo "unknown")
phrases=$(grep -oP '\d+(?= phrases available as commands)' "$source_readme" || echo "unknown")


awk '/^\| Command Syntax/,/^$/' "$source_readme" > /tmp/readme_table_raw.txt
awk -F '|' '
    NR <= 2 { print; next }
    {
        for(i=1;i<=NF;i++) gsub(/^[ \t]+|[ \t]+$/, "", $i)
        if ($5 == "✅") printf "| %s | %s | %s | %s |\n", $2, $3, $4, $5
    }
' /tmp/readme_table_raw.txt > /tmp/filtered_table.txt

# build the new content (without markers – they stay in the file)
new_content=$(cat <<EOF

My voice assistant can currently execuute **$voice_scripts** voice scripts.   
That is **$regex_patterns** regex patterns and makes a total of **$phrases** understandable phrases available as voice commands.  

| Command Syntax | Description | Example | Voice Ready |
|----------------|-------------|---------|--------------|
$(cat /tmp/filtered_table.txt)

EOF
)


if grep -q '<!-- MY_VOICE_COMMANDS_START -->' "$target_readme" && \
   grep -q '<!-- MY_VOICE_COMMANDS_END -->' "$target_readme"; then
    awk -v content="$new_content" '
        BEGIN { in_block=0 }
        /<!-- MY_VOICE_COMMANDS_START -->/ { print; in_block=1; next }
        /<!-- MY_VOICE_COMMANDS_END -->/ { print content; print; in_block=0; next }
        !in_block { print }
    ' "$target_readme" > "$target_readme.tmp" && mv "$target_readme.tmp" "$target_readme"
else
    echo "ERROR: Markers not found in $target_readme"
    exit 1
fi

rm /tmp/readme_table_raw.txt /tmp/filtered_table.txt
