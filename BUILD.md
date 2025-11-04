# Build Instructions

## Compilation sans LLM (recommandé)

Si vous n'utilisez pas le tagging LLM (qui est lent et optionnel), compilez sans la feature `llm` :

```bash
cargo build --release
```

Cela évite d'avoir besoin de `llama_cpp_sys` et de ses dépendances système.

## Compilation avec LLM (nécessite des dépendances système)

Si vous voulez utiliser le tagging LLM (avec `--use-llm`), vous devez compiler avec la feature `llm` :

```bash
cargo build --release --features llm
```

### Dépendances système requises

**Linux (Ubuntu/Debian) :**
```bash
sudo apt-get update
sudo apt-get install build-essential clang cmake
```

**Linux (Fedora/RHEL) :**
```bash
sudo dnf install gcc clang cmake
```

**macOS :**
```bash
xcode-select --install
```

**Windows :**
- Installez Visual Studio Build Tools avec le composant C++

### Erreur `stdbool.h` non trouvé

Si vous obtenez l'erreur `fatal error: 'stdbool.h' file not found`, cela signifie que les headers système C ne sont pas installés. 

**Solution :**
- Sur Linux : installez `build-essential` (voir ci-dessus)
- Sur macOS : installez Xcode Command Line Tools
- Sur Windows : installez Visual Studio Build Tools

## Utilisation

Par défaut, le tagging utilise un dictionnaire (rapide). Pour utiliser le LLM (très lent, ~30s par fichier) :

```bash
cognifs-index /path/to/dir --use-llm
```

