# Illustre - Angerô

Programa da **Suíte Angerô**, o pacote de criação do **Grupo Angerô**.

Desenho vetorial: fichas técnicas, estampas e artes — com menus, ferramentas e atalhos no estilo do Illustrator.

- Ajuda: https://github.com/wesleiad/Suite-Angero/blob/main/docs/ajuda/illustre.md
- Suíte Angerô (iniciador, assistente de IA, instaladores): https://github.com/wesleiad/Suite-Angero
- Versões compiladas: aba **Releases** deste repositório (a Suíte Angerô se atualiza sozinha a partir delas).

## Compilar

Windows, com Rust (MSVC) e Visual Studio Build Tools (C++):

```
cargo build --release --locked -p vectorcraft -p vectorcraft-cli
```

Gera `target/release/illustre.exe` e `target/release/illustre-cli.exe`.
A cada push no ramo `angero`, o GitHub Actions compila e publica uma versão nova em Releases.

## Código-base e licença

Desenvolvido por Weslei Domingues (TI) para o Grupo Angerô, a partir do código aberto do
**VectorCraft** (projeto ArtCraft, https://github.com/storytold/vectorcraft), usado como base.
Licença: MIT OU Apache-2.0, à escolha — veja `LICENSE-MIT`, `LICENSE-APACHE` e `NOTICE`.
