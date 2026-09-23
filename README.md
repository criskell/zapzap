# zapzap

Um cliente alternativo ao WhatsApp Desktop feito com o objetivo de ser um experimento *usável* tentando alcançar a velocidade máxima e o menor consumo de RAM possível.

Tentamos essa otimização *especializando* o software:

- Existir apenas para o Brasil
- Existir apenas para o cliente

## Como compilar

O motor usa uma cópia enxuta do [whatsapp-rust](https://github.com/jlucaso1/whatsapp-rust) 0.7.0 ao lado deste repositório. `scripts/fetch-fork.sh` baixa o original e aplica `patches/whatsapp-rust-0.7.0.patch`.

- Linux (interface + motor + demo): `scripts/fetch-fork.sh && scripts/build-linux.sh && scripts/build-engine.sh` (Rust nightly com `rust-src`); rode com `scripts/run-linux.sh` (ou `--demo`).
- Windows, macOS e FreeBSD: só o motor e o núcleo de demonstração (`cargo build --release -p zapzap-engine -p zapzap-core --bins`). A interface fala com o X11 por chamadas de sistema do Linux e ainda não existe para esses sistemas.

Os workflows em `.github/workflows` fazem essas compilações a cada push e publicam os binários numa release quando se cria uma tag `v*`.
