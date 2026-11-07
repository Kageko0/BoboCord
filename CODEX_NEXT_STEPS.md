# BoboCord — Auditoria v0.1 e execução orientada ao Codex

Data da revisão: 2026-11-07. Fonte: `projeto.zip` fornecido pelo mantenedor.

**Registro de execução da Fase E:** o frontend foi migrado progressivamente para
Dioxus Desktop 0.7.10. A baseline Slint foi mantida até os testes lado a lado e a
paridade no WebView2 real passarem. O domínio e o schema SQLite 001 foram
preservados. Os resultados atuais, incluindo limitações de ambiente e consumo,
estão em [docs/VALIDATION.md](docs/VALIDATION.md). As descrições de Slint/CI
ausente abaixo representam a auditoria original do ZIP, não o estado atual.
**Atualização F1:** paginação SQL por cursor, cache/DOM limitado, ancoragem,
rascunhos e navegação persistidos, regressões/carga e CI foram implementados
conforme `BOBOCORD_CODEX_FGH.md`. A migração 001 continua intacta; a 002 foi
reservada para estado local. O plano histórico de identidades que cita 002
deverá usar 003 ou posterior. Resultados reais estão em `docs/VALIDATION.md`.
As fases G/H e posteriores não foram executadas.

## 1. Estado auditado

O repositório contém workspace Rust com `apps/desktop`, `crates/bobo-core` e `crates/bobo-storage`.

- `bobo-core`: ID UUID, Space, Channel (somente Text), Message, LocalProfile, Preferences, validações, Repository, Command e Service.
- `bobo-storage`: SQLite via rusqlite, migração `001_foundation.sql`, CRUD, restrições, testes de integração.
- `apps/desktop`: UI Slint, controller `application.rs`, worker exclusivo para I/O via `mpsc`, testes de callbacks.
- `ui/`: janela Slint, NavItem e tokens.
- `docs/VALIDATION.md`: informa nove testes e validação local Windows; não é prova de verificação no ambiente desta auditoria.
- **Lacuna no ZIP**: `docs/VALIDATION.md` cita `.github/workflows/ci.yml`, mas o arquivo não consta no pacote fornecido.
- Ainda não há P2P, rede, voz, vídeo, captura ou nuvem.

Preservar todo código útil. **Não reiniciar o projeto nem perder bancos locais existentes**.

## 2. Decisão arquitetural

Migrar o frontend para **Dioxus Desktop 0.7.x**, usando renderizador WebView mantido (WebView2 no Windows, WebKitGTK no Linux), RSX e CSS modular. A versão exata deve ser escolhida e fixada no `Cargo.lock` após verificar compatibilidade e documentação oficial. Não apostar o MVP no renderer Blitz/WGPU experimental. Não iniciar Tokio ou WGPU na UI por conveniência.

Caminho planejado:

```text
Dioxus Desktop (RSX + CSS, componentes)
  -> Estado/ações de aplicação tipadas
    -> bobo-core (domínio independente)
      -> bobo-storage (SQLite com worker)
  -> posteriormente bobo-network (P2P)
  -> posteriormente bobo-media (WebRTC)
```

A UI é uma camada substituível. `bobo-core` não importa Dioxus nem SQLite. Renderização e animação não podem bloquear armazenamento nem futuras tarefas de rede.

## 3. Fase E — Migração controlada para Dioxus (prioridade imediata)

1. Registrar estado atual do repositório e baseline dos testes; não apagar arquivos antes de comprovar equivalência.
2. Separar `Application`, `Request` e `Reply` de `apps/desktop/src/application.rs` para uma camada `bobo-app` **somente se fizer sentido**; caso contrário, manter na aplicação, mas eliminar dependências Slint dessa lógica.
3. Substituir strings mágicas (`"create-space"`, `"send"` etc.) por enums tipados para eventos/comandos; preservar semântica e mensagens de erro.
4. Criar shell Dioxus com componentes `AppShell`, `SpaceRail`, `ChannelSidebar`, `ChatHeader`, `MessageList`, `MessageComposer`, `ProfileDock`, `SettingsPanel` e `ModalHost`.
5. Integrar sinais (`Signal`) e estados derivados em Dioxus sem loops de polling; preservar worker SQLite fora da thread UI. Disponibilizar `busy`, `error`, `selected_space`, `selected_channel`, `draft` e demais dados ao frontend.
6. Preservar funcionalidades: criar/renomear/excluir espaços, criar/renomear/excluir canais, enviar/ler mensagens, perfil e tema claro/escuro, estados vazios, feedback e confirmação de exclusão.
7. CSS componentizado com tokens globais: superfícies, tipografia, bordas, espaços, acento, foco, hover, active, selected, disabled. Implementar UI desktop moderna estilo blocos discretos, sem cópia direta da identidade visual Discord.
8. Priorizar interação ágil: hover instantâneo, seleção evidente, foco/teclado funcionais, indicador de envio, animações CSS curtas (100–180 ms), animação reduzida para `prefers-reduced-motion`; evitar sombras pesadas, blur e animação contínua sem necessidade.
9. Otimizar: não remontar o histórico inteiro a cada tecla, evitar rerender global de Shell, carregar somente mensagens visíveis/recentes e não colocar dados volumosos no estado global.
10. Testar uso offline, navegação por teclado, DPI 100%/125%/150%, redimensionamento e escala. Controlar dependências WebView e WebKitGTK.
11. Retirar Slint, `slint-build`, `build.rs` e os arquivos `ui/*.slint` **apenas após** o novo app compilar, iniciar e passar testes equivalentes. Caso seja útil, manter Slint numa branch até aceitação.
12. Atualizar README, AGENTS, documentação, instruções Windows/Linux, CI e testes. Registrar consumo de memória em build release e debug separadamente.

**Critério de aceite E:** paridade funcional com v0.1, persistência entre reinícios, testes verdes, nenhum código Slint na distribuição final, UI moderna responsiva e ausência de queries SQLite no loop gráfico.

## 4. Fase F — Refinamento da experiência local

- Visual de comunicação: rail compacto de espaços, sidebar de canais, painel principal, header, barra de composição e dock de perfil.
- Componentes táteis para mouse e touch: hit targets adequados, estados claros e context menus; scroll fluido e cabeçalho estável.
- Atalhos básicos: Enter envia, Shift+Enter quebra linha, Ctrl+K navegação rápida futura; evitar atalhos globais nesta fase.
- Rascunhos por canal persistidos opcionalmente; seleção restaurada ao reiniciar; editar e excluir mensagens locais quando modelados no domínio.
- Paginação do histórico (por cursor usando `(created_at, id)`); limites de carga; busca local como etapa posterior.
- Cache de assets locais, avatars com fallback; WebP estático; stickers animados só quando necessários.
- Acessibilidade: labels, foco visível, navegação e reduced motion.
- Testes de estados vazios, erro do banco, operações sobre entidades inexistentes, muitos canais/mensagens e eventos rápidos.

**Critério de aceite F:** UX agradável para uso real, sem travamentos perceptíveis com histórico maior, operações persistentes e comportamento correto de seleção/rascunhos.

## 5. Fase G — Preparação P2P (antes da rede real)

- Introduzir `Identity`/`PeerIdentity` separadas do perfil visual; modelo de autoria de mensagens não pode depender de `local_profile` como FK exclusiva.
- Criar migração **002** compatível com banco 001 (não alterar 001 em produção). Definir política para mensagens pré-existentes, com testes de upgrade e rollback seguro.
- Definir `bobo-protocol`: eventos versionados, IDs de evento imutáveis, autor, timestamps UTC informativos, idempotência, ACK, ordem por canal, duplicação e reconexão.
- Contratos `Transport` e `PeerDiscovery` sem dependência no domínio. Planejar autorização por espaço e gestão de convites; documentar modelo de confiança.
- Planejar criptografia ponta a ponta em repouso/sincronização e ciclo de vida das chaves. Não confundir TLS/QUIC com E2EE das mensagens armazenadas.
- Modelar offline queue local, reenvio, persistência de estado da entrega e retransmissões com limite.

**Critério de aceite G:** esquema evolutivo testado, protocolo documentado, casos de reconexão/duplicação simulados sem rede real.

## 6. Fase H — Prova P2P real entre dois computadores

- Implementar transporte Iroh/QUIC em crate próprio, com versão estável verificada.
- Identidades de dispositivos, pareamento/convites, descoberta e conexão direta ou relay quando necessário.
- Trocar mensagens em duas redes diferentes com estados `pending`, `sent`, `delivered` e tratamento correto de falha.
- Persistência de mensagens recebidas e deduplicação por ID.
- Testes de LAN, redes separadas, timeout, reconexão, reenvio, perda de link, NAT/CGNAT e peers offline.
- Exibir honestamente quando relay foi necessário; serviço de suporte não pode ser apresentado como zero infraestrutura em todos os cenários.

**Critério de aceite H:** duas máquinas reais conseguem trocar mensagens, reconectar e recuperar envios pendentes sem perder/duplicar registros.

## 7. Etapas posteriores (não desenvolver junto da migração)

1. Voz e presença: WebRTC/Opus, controle de ganho, ruído, eco, push-to-talk, indicadores de conexão.
2. Vídeo e compartilhamento: webcam, seleção de monitor/janela, qualidade adaptativa, APIs nativas Windows/Linux.
3. BoboReplay: buffer circular comprimido até 50 s, seleção de monitor, captura de áudio separada, atalho, exportação e testes de proteção de conteúdo.
4. Oracle opcional: arquivos efêmeros, sincronização offline criptografada e limpeza automática dentro de cotas reais.
5. Eventos, playlists autorizadas, plugins, bots e Android.

## 8. Problemas e pendências específicas identificadas

- `messages.author_id` usa FK para `local_profile(id)` — requer evolução antes de múltiplos autores P2P.
- `Repository::messages(channel_id)` carrega todas as mensagens — paginar.
- `apps/desktop/src/worker.rs` reconstrói os modelos completos após cada operação — evitar caminho equivalente no Dioxus; atualização direcionada.
- `Request.action` e `Request.id` são strings — preferir comandos tipados.
- `Application` administra seleção e rascunhos em memória — decidir persistência/escopo.
- `Theme` contém somente `Dark` e `Light` — manter inicialmente, expandir por tokens CSS depois.
- CI citada mas `.github/workflows/ci.yml` ausente no ZIP.
- Linux, release e grandes volumes não foram demonstrados no arquivo enviado.

## 9. Validação obrigatória antes de declarar pronto

Executar no ambiente do Codex, quando as ferramentas estiverem disponíveis:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace --locked
```

Executar também `dx serve --platform desktop` ou equivalente documentado na versão instalada e verificar abertura manual do executável. Windows e Linux devem ser testados independentemente; se não for possível, registrar "não verificado". Não inventar benchmark, screenshot, teste, configuração de dependências ou sucesso da CI.

Atualizar `docs/VALIDATION.md` com comandos realmente executados, números de testes, tempos/ambiente, pendências e arquivos alterados. Não usar `unwrap()` para operações usuais de produção.

## 10. Prompt operacional para o Codex

> Inspecione o workspace atual e leia README.md, AGENTS.md, docs/VALIDATION.md e este documento. Preserve o domínio Rust, SQLite e testes. Implemente **somente a Fase E: migração Slint -> Dioxus Desktop**, com interface moderna de blocos discretos e comportamento tátil/reativo. Reutilize a lógica de `Application` e `Service`, introduza ações tipadas, crie componentes RSX e CSS tokenizado, preserve integralmente os CRUDs, mensagens, perfil e tema. Evite WebRTC, Iroh, Oracle, captura ou plugins nessa entrega. Faça a migração em passos verificáveis; não exclua Slint antes da paridade. Acrescente o workflow CI ausente, atualize README/AGENTS/VALIDATION e execute todos os checks disponíveis. Ao concluir, detalhe arquivos modificados, testes, limitações, compatibilidade Windows/Linux e decisões arquiteturais. Não faça push/deploy sem autorização.
