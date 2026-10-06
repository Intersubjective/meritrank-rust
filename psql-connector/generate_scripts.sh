FOLDER=$1
if [ ! -d "$FOLDER" ]; then
  echo "Invalid arguments"
  exit 1
fi

VER=$( cargo read-manifest | jq -r '.version' )

#  Upgrade paths for `ALTER EXTENSION pgmer2 UPDATE`. The extension script is a
#  full CREATE OR REPLACE of every function, so the same file serves as the
#  update script from any earlier version. Decrementing only the last version
#  component (as this script used to) gave no path at all across a minor bump
#  (0.8.3 -> 0.9.0 looked for 0.9.-1 ... 0.9.-4), and Tentura runs
#  `ALTER EXTENSION pgmer2 UPDATE` at startup, which then fails.
#  Paths are generated from every earlier patch of this minor and from patches
#  0..19 of the two previous minors. A major bump (minor 0, major > 0) also
#  needs the previous major's last minors: list them in EXTRA_UPGRADE_FROM.
MAJOR=${VER%%.*}
REST=${VER#*.}
MINOR=${REST%%.*}
PATCH=${REST#*.}

FROMS=""
p=0
while [ "$p" -lt "$PATCH" ]; do
  FROMS="$FROMS $MAJOR.$MINOR.$p"
  p=$((p + 1))
done
for dm in 1 2; do
  m=$((MINOR - dm))
  [ "$m" -ge 0 ] || continue
  p=0
  while [ "$p" -le 19 ]; do
    FROMS="$FROMS $MAJOR.$m.$p"
    p=$((p + 1))
  done
done
FROMS="$FROMS ${EXTRA_UPGRADE_FROM:-}"

[ -d extension ] || mkdir extension
sed 's/CREATE  FUNCTION/CREATE OR REPLACE FUNCTION/g' "$FOLDER/pgmer2--$VER.sql" > "extension/pgmer2--$VER.sql"
cat extension/pgmer2--$VER.sql
for FROM in $FROMS; do
  cp extension/pgmer2--$VER.sql "extension/pgmer2--$FROM--$VER.sql"
done
cp  "$FOLDER/pgmer2.control" extension/
