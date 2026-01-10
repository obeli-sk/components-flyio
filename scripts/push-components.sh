#!/usr/bin/env bash

# Pushes all WASM components to the Docker Hub and updates obelisk-oci.toml

set -exuo pipefail

OBELISK_TOML_DIR_VALUE="${PWD}"
PREFIX="docker.io/getobelisk/components_${PWD##*/}_"
TAG="$1"

SOURCE_TOML_FILE="obelisk-local.toml"
TARGET_TOML_FILE="obelisk-oci.toml"
push() {
    RELATIVE_PATH=$1
    FILE_NAME_WITHOUT_EXT=$(basename "$RELATIVE_PATH" | sed 's/\.[^.]*$//')
    OCI_LOCATION="${PREFIX}${FILE_NAME_WITHOUT_EXT}:${TAG}"
    echo "Pushing ${RELATIVE_PATH} to ${OCI_LOCATION}..."
    if [ "$TAG" != "dryrun" ]; then
        OUTPUT=$(obelisk client component push "$RELATIVE_PATH" "$OCI_LOCATION")
    else
        OUTPUT="dryrun"
    fi
    # Replace the old location with the actual OCI location
    sed -i -E "/name = \"${FILE_NAME_WITHOUT_EXT}\"/{n;s|location\..*\"|location.oci = \"${OUTPUT}\"|}" "$TARGET_TOML_FILE"
}

# Build components
just build
cp "$SOURCE_TOML_FILE" "$TARGET_TOML_FILE"


while IFS= read -r line; do
  [[ $line != location.path* ]] && continue

  # extract quoted path
  raw_path=${line#*\"}
  raw_path=${raw_path%\"*}

  # interpolate ${OBELISK_TOML_DIR}
  path=${raw_path//\$\{OBELISK_TOML_DIR\}/$OBELISK_TOML_DIR_VALUE}

  push $path

done < "$TARGET_TOML_FILE"


echo "All components pushed and TOML file updated successfully."
