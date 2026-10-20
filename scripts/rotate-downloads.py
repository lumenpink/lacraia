#!/usr/bin/env python3
"""
scripts/rotate-downloads.py — Gerenciador de Rotação e Retenção Soberana da Banca
Garante a retenção estrita de até 5 versões estáveis e até 10 compilações nightlies
no manifesto index.json e no storage R2 para cada aplicação do ecossistema.
"""

import argparse
import json
import os
import sys
from datetime import datetime, timezone

MAX_STABLES = 5
MAX_NIGHTLIES = 10

def prune_manifest_data(data):
    """
    Trunca listas de versões para os limites máximos permitidos:
    - stables: máximo 5 versões
    - nightlies: máximo 10 versões
    """
    if "stables" in data and isinstance(data["stables"], list):
        data["stables"] = data["stables"][:MAX_STABLES]
        if data["stables"] and not data.get("stable_latest"):
            data["stable_latest"] = data["stables"][0].get("version", "")

    if "nightlies" in data and isinstance(data["nightlies"], list):
        data["nightlies"] = data["nightlies"][:MAX_NIGHTLIES]
        if data["nightlies"] and not data.get("nightly_latest"):
            data["nightly_latest"] = data["nightlies"][0].get("version", "")

    return data

def prune_manifest_file(file_path):
    if not os.path.exists(file_path):
        print(f"Erro: Arquivo {file_path} não encontrado.", file=sys.stderr)
        return False

    with open(file_path, "r", encoding="utf-8") as f:
        data = json.load(f)

    data = prune_manifest_data(data)

    with open(file_path, "w", encoding="utf-8") as f:
        json.dump(data, f, indent=2, ensure_ascii=False)
        f.write("\n")

    print(f"✔ Manifesto {file_path} podado com sucesso (<= {MAX_STABLES} stables, <= {MAX_NIGHTLIES} nightlies).")
    return True

def add_entry(manifest_path, channel, version, url, sha256="", tarball=""):
    data = {"app": "app", "stables": [], "nightlies": []}
    if os.path.exists(manifest_path):
        with open(manifest_path, "r", encoding="utf-8") as f:
            try:
                data = json.load(f)
            except Exception:
                pass

    today = datetime.now(timezone.utc).strftime("%Y-%m-%d")
    entry = {
        "version": version,
        "url": url,
        "date": today,
        "sha256": sha256
    }
    if tarball:
        entry["tarball"] = tarball

    if channel == "stable":
        data["stable_latest"] = version
        stables = [e for e in data.get("stables", []) if e.get("version") != version]
        data["stables"] = [entry] + stables
    else:
        data["nightly_latest"] = version
        nightlies = [e for e in data.get("nightlies", []) if e.get("version") != version]
        data["nightlies"] = [entry] + nightlies

    data = prune_manifest_data(data)

    os.makedirs(os.path.dirname(os.path.abspath(manifest_path)), exist_ok=True)
    with open(manifest_path, "w", encoding="utf-8") as f:
        json.dump(data, f, indent=2, ensure_ascii=False)
        f.write("\n")

    print(f"✔ Adicionada versão {version} ao canal {channel} em {manifest_path}.")
    return True

def main():
    parser = argparse.ArgumentParser(
        description="Gerenciador de Rotação e Retenção Soberana da Banca (5 estáveis, 10 nightlies)."
    )
    parser.add_argument("--prune-manifest", help="Caminho do arquivo index.json para podar")
    parser.add_argument("--manifest", help="Caminho do index.json alvo para adicionar versão")
    parser.add_argument("--channel", choices=["stable", "nightly"], help="Canal da release")
    parser.add_argument("--version-str", help="Identificador semver ou atômico da versão")
    parser.add_argument("--url", help="URL do binário no storage R2")
    parser.add_argument("--sha256", default="", help="Hash SHA-256 do binário")
    parser.add_argument("--tarball", default="", help="URL do tarball comprimido (opcional)")

    args = parser.parse_args()

    if args.prune_manifest:
        if not prune_manifest_file(args.prune_manifest):
            sys.exit(1)
        return

    if args.manifest and args.channel and args.version_str and args.url:
        add_entry(
            manifest_path=args.manifest,
            channel=args.channel,
            version=args.version_str,
            url=args.url,
            sha256=args.sha256,
            tarball=args.tarball
        )
        return

    parser.print_help()

if __name__ == "__main__":
    main()
