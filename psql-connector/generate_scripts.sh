FOLDER=$1
if [ ! -d "$FOLDER" ]; then
  echo "Invalid arguments"
  exit 1
fi

VER=$( cargo read-manifest | jq -r '.version' )

VER0=$( echo $VER | awk -F. -v OFS=. '{$NF -= 1; print}' )
VER1=$( echo $VER | awk -F. -v OFS=. '{$NF -= 2; print}' )
VER2=$( echo $VER | awk -F. -v OFS=. '{$NF -= 3; print}' )
VER3=$( echo $VER | awk -F. -v OFS=. '{$NF -= 4; print}' )

[ -d extension ] || mkdir extension
sed 's/CREATE  FUNCTION/CREATE OR REPLACE FUNCTION/g' "$FOLDER/pgmer2--$VER.sql" > "extension/pgmer2--$VER.sql"
cat extension/pgmer2--$VER.sql
cp  extension/pgmer2--$VER.sql extension/pgmer2--$VER3--$VER.sql
cp  extension/pgmer2--$VER.sql extension/pgmer2--$VER2--$VER.sql
cp  extension/pgmer2--$VER.sql extension/pgmer2--$VER1--$VER.sql
cp  extension/pgmer2--$VER.sql extension/pgmer2--$VER0--$VER.sql
cp  "$FOLDER/pgmer2.control" extension/
