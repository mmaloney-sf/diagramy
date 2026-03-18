#!/bin/bash
# Build examples.json from examples directory

echo "Building examples.json..."

echo "{" > web/examples.json
echo '  "examples": [' >> web/examples.json

first=true
for file in examples/*.dgmy; do
    if [ "$first" = true ]; then
        first=false
    else
        echo "," >> web/examples.json
    fi

    basename=$(basename "$file" .dgmy)
    filename=$(basename "$file")

    echo "    {" >> web/examples.json
    echo "      \"id\": \"$basename\"," >> web/examples.json
    echo "      \"name\": \"$filename\"," >> web/examples.json
    echo -n "      \"content\": " >> web/examples.json

    # Escape the content for JSON
    python3 -c "import json, sys; print(json.dumps(open('$file').read()))" >> web/examples.json

    echo -n "    }" >> web/examples.json
done

echo "" >> web/examples.json
echo "  ]" >> web/examples.json
echo "}" >> web/examples.json

echo "Done! Created web/examples.json"

