# Validação atual — Fase F1 / qualidade e eficiência local

Registro de 7 de novembro de 2026. Ambiente: Windows x86_64/MSVC, Rust/Cargo
1.95.0, Python 3.11.0 e WebView2 157.0.4341.42. Executada somente a F1 de
`BOBOCORD_CODEX_FGH.md`. Nenhum push, deploy, serviço Oracle, rede P2P, identidade
remota, voz ou captura foi implementado. Os registros anteriores estão abaixo.

## Baseline, migração e compatibilidade

- Lidos integralmente README, AGENTS, CODEX_NEXT_STEPS, VALIDATION e
  BOBOCORD_CODEX_FGH; inspecionado o checkout e registrado `git status`.
- Antes de editar, passaram `cargo fmt --all -- --check`, Clippy com
  `-D warnings`, `cargo test --workspace --locked` (**14 testes**) e build release.
- SHA256 da **001 intacta**, confirmado antes/depois:
  `07E389517DF89271D3A638D92706B66029D9591639A083E0F507753D6733FC34`.
- Nova `002_local_ui.sql`: somente `channel_drafts` e `local_navigation`, com
  FKs/cascatas. Bancos v1 recebem 002 em transação. Instalação nova aplica
  001 + perfil + 002 na mesma transação. Versões futuras (>2) são recusadas.
- Upgrade v1→v2 preservou IDs, perfil, preferência, corpo e timestamps em testes;
  reabrir é idempotente. Falha da 002 fez rollback de DDL/versão sem tocar a v1.
- Não houve alteração de autores/identidades. A fase G futura deverá usar 003
  ou posterior, sem reutilizar a 002. Bancos pessoais não foram abertos nem
  movidos; os executáveis de teste usaram exclusivamente overrides em `target/`.

## Implementação e decisões

`Repository::messages_page(channel, before, limit)` retorna `MessagePage` com
mensagens cronológicas, `before` e `has_more`. SQL filtra o canal e usa
`(created_at,id) < (?,?) ORDER BY created_at DESC,id DESC LIMIT ?`, sem OFFSET,
COUNT total ou consulta integral. Padrão: 50; máximo defensivo: 200; lê no máximo
limit+1 para descobrir `has_more` e inverte somente o resultado dessa página.
A abordagem corresponde às [consultas por tuplas documentadas pelo SQLite](https://www.sqlite.org/rowvalue.html).
O índice `messages_by_channel(channel_id,created_at,id)` da 001 já serve ao
range e à ordem reversa: nenhum índice redundante foi criado.

`Service::metadata_snapshot` separa navegação/perfil/tema de mensagens. O
`snapshot()` de conveniência usa somente a página inicial; `messages()` legado
fica limitado a 200 e não deve ser usado como API de histórico completo.
O caminho desktop usa páginas reais. `bobo-app` deduplica por UUID, ordena por
cursor e limita cache/DOM a **300 mensagens**. Ao buscar muito para trás, retira
as mais novas da janela e oferece **Voltar às recentes**, sem fingir continuidade
quando há um intervalo não carregado. Rascunho e canal não mudam ao paginar.

`history.js` mantém a âncora visível por ID/posição antes e depois da mutação.
Ao acrescentar texto, só segue o fim quando a distância anterior era ≤72 px.
Trocar canal/solicitar recentes tem intenção explícita de reset. Não usa timers
de render, polling, animação de rolagem ou consultas de domínio no browser;
MutationObserver e requestAnimationFrame só atuam após alterações reais do DOM.

Rascunhos: sinal local imediato, fila coalescida por canal e debounce de **250 ms
no worker**, usando `recv_timeout` apenas enquanto há edição pendente. Flush em
ações de primeiro plano e encerramento normal. A fila reserva espaço para wake
e comando, evitando erro ao digitar e enviar imediatamente. Erros de autosave
usam estado separado e não desmontam o histórico nem desbloqueiam outro comando.
Texto é preservado sem trim até 65.536 caracteres, inclusive rascunhos ainda
inválidos para o envio de 4.000 caracteres. Escrita em lote é transacional.
Inserir mensagem e remover seu rascunho também é uma transação; uma falha mantém
o rascunho. Exclusão de canal/espaço limpa os rascunhos e invalida seleções por FK.
Um autosave atrasado para canal já excluído é ignorado sem criar órfão.

UI: menu contextual local para espaços/canais com botão direito/Shift+F10,
autofoco, setas, Escape, retorno de foco e ações reais de renomear/excluir.
Erros de formulário recebem `aria-invalid`/descrição; compositor tem ajuda
associada. Foco, hover, disabled e reduced motion existentes foram preservados.
Nenhuma dependência Rust ou biblioteca visual nova foi adicionada.

## Checks e regressões executados

| Comando | Resultado final |
| --- | --- |
| `cargo fmt --all -- --check` | Passou |
| `cargo clippy --workspace --all-targets -- -D warnings` | Passou, sem warnings |
| `cargo test --workspace --locked` | **21 testes**, todos passaram, nenhum ignorado |
| `cargo build --workspace --locked` | Passou, debug |
| `cargo build --workspace --release --locked` | Passou, release |
| `cargo test -p bobo-storage --test pagination --locked -- --nocapture` | 4 testes passaram, com plano/tempo impressos |
| `python scripts/desktop_smoke.py flow`, `reopen`, `keyboard` | Regressão nativa v0.1 passou na build F1 final |
| `python scripts/f1_smoke.py scroll`, `context`, `write-close <PID>`, `read` | Passaram no WebView2 real com fixture de carga |
| `python scripts/startup_f1.py <binário> <diretório>` | 4 amostras reais, tabela abaixo |
| `scripts/measure-desktop.ps1 ...` | 4 amostras reais, tabela abaixo |
| Análise dos scripts Python por `ast.parse`, `git diff --check` | Passaram |

Contagem: 3 testes de domínio, 5 de persistência originais, 4 novos de
paginação/migração/estado, 7 da aplicação/worker e 2 Dioxus. Nenhum teste foi
removido. O teste anterior da janela foi ajustado ao tamanho de página de F1.
Cobertura adicional:

- 10.000 mensagens em quatro canais, 2.500 por canal, timestamps repetidos;
  percorreu todas as páginas do canal sem duplicações/omissões mesmo inserindo
  uma mensagem recente após obter o primeiro cursor. Limites 0/UINT_MAX e
  ordenação/isolamento conferidos.
- `EXPLAIN QUERY PLAN`: `SEARCH messages USING INDEX messages_by_channel
  (channel_id=? AND (created_at,id)<(?,?))`, sem sort temporário.
- Na execução com `--nocapture`, **50 páginas percorridas em 27,422 ms**.
  Esse tempo inclui asserts e uma inserção; não é benchmark puro de SELECT.
- 700 mensagens com timestamps repetidos na aplicação: navegação completa,
  deduplicação e janela ≤300 em todas as etapas, retorno às recentes e recarga.
- Upgrade 001→002, rollback da 002, persistência entre conexões, rollback de
  texto+seleção inválida, cascata e atomicidade de envio/limpeza de rascunho.
- Rajada de 1.000 edições coalescidas, último texto salvo por shutdown sem esperar
  debounce; autosave sem navegar e edição vazia removendo o rascunho.
- Callback Dioxus de digitação seguido imediatamente por envio; identidade dos
  nós/histórico permanece estável durante digitação.

O Clippy inicialmente pediu reduzir o tamanho do enum de resposta e remover
aninhamento/mutabilidade desnecessários; foram corrigidos. Um build debug
encontrou o executável de teste ainda aberto (lock do Windows); a janela foi
fechada normalmente e o build seguinte passou. Não foram suprimidos lints.

## Testes no WebView2 real

`seed_load.py` criou uma fixture v1 nova com **10 mil mensagens em quatro canais**
(9 mil no canal ativo e mil nos outros), alturas variáveis e timestamps iguais.
O app migrou essa fixture automaticamente, abriu com **50 linhas DOM** e chegou
a **no máximo 300** durante oito cargas de anteriores.

Desvio da âncora medido por `getBoundingClientRect`: **0,015625 px** ao carregar
anteriores e **0,015625 px** ao acrescentar mensagem com a leitura longe do fim
(asserção aceita <2 px). Não saltou para baixo; enviar já perto do fim acompanhou
a mensagem. Voltar às recentes restaurou 50 linhas. Identidade dos nós permaneceu
igual durante digitação/autosave. Shift+F10, foco no menu, ArrowDown e Escape
passaram. Também passaram os fluxos anteriores de CRUD, validação, tema/perfil,
cascatas, Tab no modal, Enter/Shift+Enter e reduced motion.

O teste de persistência mudou para outro canal, digitou texto multilinha/emoji,
aguardou somente 30 ms e enviou WM_CLOSE **antes do debounce de 250 ms**. Reabrir
restaurou o canal e a última edição; o rascunho do outro canal também permaneceu.
Os logs finais de testes nativos ficaram sem erros. Capturas reais `f1-*.png`
foram geradas em `target/` e inspecionadas. CDP foi habilitado só nos processos
de teste por variável de ambiente; não no aplicativo normal. Esses testes
precisaram de execução fora das restrições de subprocessos/rede do sandbox.

## Métricas F1 — amostras reais, não garantias

Windows/WebView2 acima, janela em repouso 3 segundos após abrir, **sem porta de
depuração durante a amostra de memória/CPU**. Todas as árvores tiveram 8 processos.
Dados vazios e fixture de 10 mil mensagens, usando os mesmos diretórios nas
comparações debug/release. Cada amostra durou cerca de 10,1 s.

| Métrica | Debug vazio | Release vazio | Debug 10 mil | Release 10 mil |
| --- | ---: | ---: | ---: | ---: |
| Working set principal (MiB) | 31,29 | 26,88 | 31,90 | 26,97 |
| Bytes privados principal (MiB) | 5,56 | 5,33 | 5,91 | 5,50 |
| CPU principal na amostra (s) | 0 | 0 | 0 | 0 |
| Soma working set árvore (MiB) | 377,82 | 364,45 | 381,69 | 366,95 |
| Soma bytes privados árvore (MiB) | 166,73 | 165,12 | 170,65 | 171,74 |
| CPU árvore na amostra (s) | 0 | 0 | 0 | 0 |
| Duração da amostra (s) | 10,11 | 10,10 | 10,10 | 10,11 |
| DOM inicial (linhas) | 0 | 0 | 50 | 50 |

Tempos de abertura foram medidos **separadamente**, com porta CDP local, do início
do processo até o renderer pronto. Incluem overhead de conexão/probe Python;
uma observação por cenário, sem pretensão de benchmark estatístico:

| Cenário | Debug | Release |
| --- | ---: | ---: |
| Vazio | 652,13 ms | 678,89 ms |
| 10 mil mensagens | 661,17 ms | 562,51 ms |

O primeiro debug criou os perfis WebView e o cenário de carga fez upgrade v1→v2;
o release posterior reutilizou cache/estado. Não inferir superioridade de build
ou startup quente desses quatro números. Working sets somados podem duplicar
páginas compartilhadas; bytes privados não são RAM física residente. CPU zero
é a resolução observada nessa janela, não promessa de consumo zero.
**A meta inicial de RAM de 80–150 MB continua não atendida por essas medidas.**
JSONs completos estão em `target/f1-metrics-*.json`; nenhum dado foi enviado.

## Arquivos alterados e CI

- `crates/bobo-core/src/lib.rs`: cursores/páginas, estado local e separação de metadata.
- `crates/bobo-storage/src/lib.rs`, nova `migrations/002_local_ui.sql` e
  `tests/pagination.rs`: SQL limitado, migração e regressões/carga. A 001 e os
  testes originais desse crate foram preservados.
- `crates/bobo-app/src/{lib,worker}.rs`, `tests/parity.rs`: janela limitada,
  deduplicação, navegação persistida e debounce/flush no worker.
- `apps/desktop/src/state.rs`, `tests.rs`, componentes chat/channels/spaces/shell/
  modal/mod e novo `context_menu.rs`: sinais, autosave, acessibilidade/menu e testes.
- `apps/desktop/assets/history.js`, chat.css/forms.css: ancoragem e estilos locais.
- Scripts novos `seed_load.py`, `f1_smoke.py`, `startup_f1.py`; atualizado
  `measure-desktop.ps1` para separar bytes privados/CPU do processo principal.
- `.gitignore`: cache Python ignorado; nenhum pacote adicional foi instalado.
- `.github/workflows/ci.yml` **existe no checkout**. Confirmada matriz Windows e
  Ubuntu 24.04 com GTK3/WebKitGTK 4.1, fmt/Clippy/test/build; acrescentado build
  release e timeout. O Git ainda mostra o workflow não rastreado, como o restante
  da implementação neste checkout. Não houve staging, commit ou run remoto.
- README, AGENTS, CODEX_NEXT_STEPS, BOBOCORD_CODEX_FGH e este registro atualizados
  sem apagar o histórico. `projeto.zip` não foi alterado/reempacotado.

## Limitações e próxima ação

Não verificados: Linux/macOS, CI remota, `dx` (não instalado), DPI nativo variado,
leitor de tela, crash/desligamento forçado, disco cheio/perda de energia, conflito
de edições entre múltiplas instâncias e benchmarks repetidos sob carga contínua.
Escalas simuladas e assets locais foram cobertos pela regressão nativa anterior;
não equivalem a DPI nativo ou desconectar fisicamente a máquina.

O banco não é criptografado. Crash pode perder a edição ainda dentro do debounce;
erros de armazenamento em shutdown são relatados no stderr. Rolagem/página exata
não persistem entre processos. O DOM é limitado, mas não implementa virtualização
arbitrária de milhares de linhas. Autoria continua local; não há G/H ou serviços
Oracle. Os bancos de usuário não foram usados para as medições.

Próxima ação sugerida: revisão manual F1 e execução independente Linux/CI antes
de aceitar esta etapa; qualquer início de G/H requer nova solicitação.

---

## Histórico da Fase E / Dioxus Desktop

Os resultados e limites abaixo descrevem a etapa anterior e não o estado F1 atual.

Registro de 7 de novembro de 2026. Windows x86_64/MSVC, Rust/Cargo 1.95.0,
Python 3.11.0 e Microsoft Edge WebView2 Runtime 157.0.4341.42.
Somente a Fase E de `CODEX_NEXT_STEPS.md` foi executada. Não houve push ou deploy.

## Migração progressiva e preservação

1. Lidos integralmente README, AGENTS, este registro e CODEX_NEXT_STEPS.
   Registrado o estado do Git e hashes SHA256 dos fontes antes de editar.
   Baseline `cargo test --workspace --locked`: **9 testes passaram**.
2. Extraída a aplicação para `bobo-app`: ações tipadas, Request/Reply, seleção,
   rascunhos e worker sem dependência de UI. O adaptador Slint temporário passou
   pelos mesmos 9 testes originais depois da extração.
3. Criado `bobocord-desktop-next` temporário, lado a lado com o frontend anterior.
   Testes do workspace nessa etapa: **15 passaram** (8 domínio/SQLite, 1 Slint,
   4 aplicação/worker e 2 Dioxus).
4. Compilado e aberto o Dioxus no Windows. Testes no DOM real do WebView2
   comprovaram CRUDs, mensagens, erros, rascunhos, perfil e tema. O processo foi
   fechado normalmente e reaberto com o mesmo banco; a persistência, cancelamento
   e exclusões com cascata passaram.
5. Somente após essa paridade, o frontend novo foi promovido a
   `bobocord-desktop`. Slint, slint-build, o backend de testes Slint, build.rs,
   o adaptador temporário e `ui/*.slint` foram retirados. O pacote temporário
   também foi removido. Os testes específicos do renderer foram portados para
   Dioxus, incluindo o cenário original e uma regressão sobre digitação/histórico.
6. Verificados novamente os hashes: **todos os arquivos de `bobo-core` e
   `bobo-storage`, incluindo migração 001 e testes, são idênticos ao baseline**.
   Nenhuma migração nova, alteração de schema ou conversão de banco foi feita.
   Caminho padrão do banco e override `BOBOCORD_DATA_DIR` continuam iguais.

O baseline Slint foi copiado para `target/phase-e-baseline` antes da substituição,
para comparação local. É um artefato de validação ignorado pelo Git, não parte
da distribuição final. A árvore de fontes e o lockfile finais não contêm Slint.
Os bancos usados nestes testes estão exclusivamente em `target/`; bancos pessoais
não foram abertos, movidos nem convertidos.

## Comandos e resultados da Fase E

| Comando | Resultado real |
| --- | --- |
| `cargo test --workspace --locked` antes da migração | 9 testes passaram |
| `cargo test --workspace --locked` com ambos os frontends | 15 testes passaram |
| `cargo fmt --all` | Fontes formatados |
| `cargo fmt --all -- --check` | Passou |
| `cargo clippy --workspace --all-targets -- -D warnings` | Passou, sem warnings |
| `cargo test --workspace` final | **14 testes passaram**, nenhum ignorado |
| `cargo build --workspace --locked` | Passou, executável debug |
| `cargo build --workspace --release --locked` | Passou, executável release |
| `python scripts/desktop_smoke.py flow` | Passou no WebView2 real, antes e após a promoção |
| `python scripts/desktop_smoke.py reopen` | Passou após fechar/reabrir o processo com o mesmo banco |
| `python scripts/desktop_smoke.py keyboard` | Passou no executável final |
| `scripts/measure-desktop.ps1 -BoboProcessId <PID> -Label debug` | Amostra real registrada abaixo |
| `scripts/measure-desktop.ps1 -BoboProcessId <PID> -Label release` | Amostra real registrada abaixo |
| `cargo tree --workspace -e normal --prefix none` | Sem Slint, WGPU, Iroh, WebRTC ou Oracle ativos |
| `git diff --check` e análise sintática Python por `ast.parse` | Passaram |

Os primeiros builds usaram `--offline` depois de baixar dependências oficiais
com `cargo --config net.offline=false fetch`. O cache inicial não continha
Dioxus e a variável de ambiente Cargo forçava offline. O download precisou de
execução fora das restrições de rede do sandbox. Não foi alterada configuração
global do usuário.

O CLI `dx` não estava instalado: usou-se a execução equivalente documentada por
Cargo/executável. O binário final foi aberto em debug e release. Não se declara
que `dx serve` tenha sido executado.

## Decisões e incompatibilidades resolvidas

- Dioxus/dioxus-desktop **0.7.10**, fixados; renderer WebView mantido, sem Blitz.
  As [dependências de plataforma oficiais](https://dioxuslabs.com/learn/0.7/getting_started/#platform-specific-dependencies)
  orientaram as instruções Windows/Linux.
- Dioxus Desktop 0.7.10 não compilou com todas as features padrão desativadas:
  `edits.rs` referencia Tokio sem proteção de feature. Após habilitar
  `tokio_runtime`, a tentativa de usar a entrada bloqueante sem runtime falhou
  na abertura porque o WebView obtém `tokio::runtime::Handle::current()`.
  Foi adotada a entrada oficial `launch_virtual_dom`, que cria o runtime interno
  exigido pelo framework. A aplicação não cria runtime próprio e o worker,
  domínio e armazenamento permanecem sem dependência Tokio.
- O build release revelou chamadas não protegidas a `open_devtools` e
  `close_devtools` no Dioxus. A dependência direta Wry 0.53.5 habilita somente
  `wry/devtools`, disponibilizando esses métodos. Não se habilitou o devserver
  Dioxus nem depuração remota na aplicação normal. O build release seguinte passou.
- `bobo-app` fez sentido porque ambos os frontends consumiram a mesma aplicação
  durante a transição. IDs, validações e semântica dos comandos existentes não
  foram alterados. Enums substituem strings mágicas no caminho final.
- `Signal`s separados para navegação, histórico, perfil, tema, status, modal e
  rascunho. Apenas valores alterados são publicados. Mensagens usam keys UUID.
  O teste de digitação confere estabilidade do histórico e das mutações DOM.
- A UI recebe as 200 mensagens recentes e amplia em 200 sob ação explícita.
  O worker ainda faz a consulta integral da API existente antes de recortar;
  isso **não é paginação SQL por cursor** nem virtualização completa.
- CSS embutido e separado em tokens/shell/chat/forms: temas, foco, hover, active,
  selected, disabled, animação curta e reduced motion, sem CDN/fontes remotas.
- O teste nativo de Tab encontrou uma saída de foco do modal. O gerenciamento
  síncrono de foco no browser foi acrescentado e os testes Tab/Escape passaram.
- Um teste SSR inicialmente esperava a forma nominal `&lt;`; o Dioxus usa
  entidades numéricas equivalentes. A asserção foi corrigida, preservando o teste
  de que conteúdo de mensagem não pode virar elemento/script executável.

## Testes finais e paridade nativa

Os **14 testes Rust** são: 3 de domínio e 5 de persistência originais inalterados,
4 de aplicação/worker (paridade tipada, retry de abertura, shutdown sem bloqueio,
janela de histórico) e 2 Dioxus (cenário original portado e digitação sem
reconstruir histórico). A cobertura do teste Slint foi mantida no teste Dioxus de
mesmo nome; não foi removida para contornar falhas.

O teste Python usa CDP em loopback, habilitado **somente no ambiente do processo
de teste**, com banco novo isolado. Executa eventos DOM e eventos de teclado no
WebView2 real. Confirmados no executável final:

- Criar e renomear espaços/canais; enviar e listar mensagens com UTC.
- Erro recuperável para mensagem vazia, preservação do texto e conteúdo escapado.
- Navegação entre canais sem misturar histórico e rascunhos por canal.
- Tema claro/escuro e perfil; persistência após encerrar/reabrir o processo.
- Cancelar exclusão sem perder dados; excluir canal preservando o outro histórico;
  excluir espaço removendo canais/mensagens associados.
- Autofoco, sete ciclos Tab presos no modal, Escape, Enter para envio,
  Shift+Enter para newline e preferência reduced motion.
- Redimensionamento para 740×480 e escalas **simuladas** 1, 1.25 e 1.5, sem
  overflow horizontal. Isso não substitui alterar o DPI real do Windows.
- Recursos observados no renderer restritos aos protocolos/endereços locais do
  Dioxus. Não houve acesso a serviços remotos no fluxo verificado.

As primeiras tentativas de abrir WebView2 sob o sandbox criaram processos sem
renderer utilizável; os testes nativos foram executados fora dessa restrição.
Logs finais ficaram sem erros. Capturas reais (`target/dioxus-*.png`) foram
geradas e inspecionadas; capturas, logs, bancos e métricas são artefatos locais
ignorados pelo Git. Não foram enviados para serviços externos.

## Memória — debug e release separados

Amostras em repouso, janela vazia, 3 segundos após abrir, sem porta de depuração,
com o mesmo diretório SQLite de teste. `measure-desktop.ps1` enumera somente a
árvore do PID fornecido, incluindo WebView2. Cada amostra durou **10,15 s**.

| Métrica | Debug | Release |
| --- | ---: | ---: |
| Processos na árvore | 8 | 8 |
| Working set do processo principal | 31,71 MiB | 29,50 MiB |
| Soma dos working sets da árvore | 372,38 MiB | 367,23 MiB |
| Soma de bytes privados da árvore | 165,79 MiB | 163,67 MiB |
| CPU acumulada na árvore durante a amostra | 0 s | 0 s |
| Tamanho do executável | 13.295.104 bytes | 6.501.888 bytes |

**A meta inicial de RAM de 80–150 MB não foi atingida pela soma de working sets.**
Não comparar estes números com a amostra antiga de um único processo Slint.
Working set somado pode duplicar páginas compartilhadas; bytes privados são
outra medida e não equivalem a RAM física residente. Amostras pontuais não são
benchmark nem garantia. GPU, startup, muitos canais e histórico extenso não
foram medidos. Arquivos JSON completos ficam em `target/dioxus-memory-*.json`.

## Arquivos e CI

- `Cargo.toml`, `Cargo.lock`: novo pacote bobo-app, Dioxus/Wry e remoção Slint.
- `crates/bobo-app/{Cargo.toml,src/lib.rs,src/worker.rs,tests/parity.rs}`:
  aplicação e worker compartilhados sem renderer.
- `apps/desktop/{Cargo.toml,src/main.rs,src/state.rs,src/tests.rs}`: executável,
  sinais/ações e testes portados.
- `apps/desktop/src/components/{shell,spaces,channels,chat,settings,modal,mod}.rs`:
  componentes solicitados; `ProfileDock` está em channels e os três componentes
  de chat estão em chat.
- `apps/desktop/assets/{tokens,shell,chat,forms}.css`: CSS local embutido.
- Removidos: `apps/desktop/build.rs`, application/worker específicos de Slint,
  `ui/app.slint`, `ui/components/nav-item.slint`, `ui/themes/tokens.slint`.
- `scripts/desktop_smoke.py`, `scripts/measure-desktop.ps1`: verificações nativas.
- `.github/workflows/ci.yml`: formatação, Clippy, testes e build com lockfile,
  Windows e Ubuntu 24.04 com GTK3/WebKitGTK 4.1. O workflow já existia **não
  rastreado** neste checkout; foi confirmado e atualizado. A auditoria anterior
  dizia que faltava no ZIP. Sua presença final foi verificada no filesystem.
- README, AGENTS, CODEX_NEXT_STEPS e este registro atualizados. O planejamento
  anterior e os resultados Slint abaixo permanecem como histórico.

## Limites e próxima ação

**Não verificados:** build/abertura Linux, CI remota, Windows limpo sem WebView2,
DPI nativo 100%/125%/150%, leitor de tela, desconexão física da Internet e carga
extensa. Nenhum resultado desses ambientes foi inferido dos testes Windows.
Os comandos e dependências propostos Linux estão no README e no workflow.
Uso offline foi validado quanto a assets locais e paridade sem serviços externos;
o teste não desligou a conexão da máquina nem inspecionou tráfego de manutenção
do runtime WebView2 do sistema.

Não foram implementados P2P, Iroh, WebRTC, Oracle, mídia, captura ou fases F–H.
O banco permanece não criptografado; rascunhos e seleção não persistem entre
processos. A API de armazenamento ainda lê o histórico completo no worker.
A licença do BoboCord continua pendente do mantenedor.

Próxima ação: revisão da Fase E no Windows e execução independente em Linux/CI,
com validação de DPI nativo e acessibilidade antes de aceitar a migração.

---

## Histórico da entrega Slint (anterior à Fase E)

Tudo abaixo registra a entrega anterior; referências a arquivos Slint, contagens
de testes, dependências e arquivos não alterados referem-se àquela etapa.

Validação local em 7 de novembro de 2026, Windows x86_64/MSVC, Rust e Cargo 1.95.0.
Esta entrega contém somente a fundação local. Não houve push, release ou deploy.

## Fases e resultados

| Fase | Entrega | Verificação executada |
| --- | --- | --- |
| A | Workspace com três pacotes e janela Slint inicial | `cargo build --workspace --offline` e `cargo test --workspace --offline`: passaram; ainda sem testes funcionais nesta fase |
| B | Domínio, SQLite, migração transacional e regras | `cargo test --workspace --offline`: 3 testes de domínio e 5 de persistência passaram |
| C | UI ligada ao domínio por thread de armazenamento | Build passou; teste adicional com callbacks Slint passou: 9 testes no total |
| D | CI Windows/Ubuntu, instruções de execução e revisão | Formatação, Clippy, testes e build final passaram; abertura real no Windows confirmada |

## Comandos finais realmente executados

| Comando | Resultado |
| --- | --- |
| `rustc --version` | `rustc 1.95.0 (59807616e 2026-04-14)` |
| `cargo --version` | `cargo 1.95.0 (f2d3ce0bd 2026-03-21)` |
| `cargo fmt --all` | Aplicou a formatação |
| `cargo fmt --all -- --check` | Passou |
| `cargo clippy --workspace --all-targets -- -D warnings` | Passou, sem warnings |
| `cargo test --workspace` | Passou: 9 testes, nenhum ignorado |
| `cargo build --workspace --locked` | Passou |
| `cargo tree --workspace -e normal --prefix none` | Dependências ativas revisadas; sem Tokio, WGPU, Iroh, WebRTC ou Oracle |

Build, testes e Clippy também passaram com `--offline`, usando o cache de crates
existente. Não foi necessário baixar dependências para os checks locais.

A primeira tentativa de fixar `rust-toolchain.toml` em `1.95.0` encontrou um
toolchain instalado somente sob o alias `stable`: rustup tentou sincronizar e
não conseguiu criar um temporário fora do workspace. O arquivo foi ajustado
para `stable`, com `rust-version = 1.95` no workspace e CI fixada em 1.95.0.
A compilação e os testes seguintes foram executados normalmente.

## Cobertura automatizada

- Domínio: nomes vazios, controles, aparagem, limites em caracteres Unicode;
  mensagens vazias, multilinha, NUL, limites e IDs estáveis.
- Persistência em arquivos de banco temporários: CRUD de espaços/canais,
  mensagens, perfil e tema após reabertura; migração idempotente.
- Integridade: unicidade por contexto, chaves estrangeiras, operações em itens
  inexistentes, cascata de canal/espaço sem afetar outros canais/espaços.
- Migrações: versão futura recusada; falha de migração reverte DDL e versão.
- UI: backend Slint headless com event loop real, usando os mesmos callbacks,
  worker, domínio e SQLite do executável. Exercita criar/renomear/excluir,
  reconciliação de seleção, enviar/listar mensagens, timestamps, isolamento por
  canal, rascunhos por canal, tema, perfil e recuperação de validação sem perder
  o texto. Reabre o banco para conferir os dados persistidos.

Os testes da UI invocam seus callbacks; não equivalem a uma campanha completa
de testes por mouse/teclado no desktop.

## Janela e event loop

Executado `target/debug/bobocord-desktop.exe` por `Start-Process`, com
`BOBOCORD_DATA_DIR` apontando para `target/smoke-data`, sem acessar dados reais
do usuário. Confirmada a janela `BoboCord · v0.1 Foundation` com Winit/software,
capturada pela API de janelas do Windows e inspecionada visualmente.

Foram conferidos o layout vazio, o histórico preenchido por fixture local de
teste (incluindo quebra de linhas) e a tela de configurações, aberta por evento
de clique na janela. O log de erro permaneceu vazio e a janela foi fechada
normalmente. Capturas e scripts temporários estão em `target/`, ignorado pelo Git.

Uma amostra de aproximadamente 10,01 segundos, após a inicialização do build
debug com uma fixture pequena, registrou working set de **46,43 MiB** e variação
de CPU acumulada de **0 segundos** na precisão fornecida pelo Windows. Isso não
é benchmark nem garantia de consumo. Não foram medidos GPU, startup quente,
históricos grandes ou build release. A primeira observação do layout vazio,
três segundos após abrir, registrou aproximadamente 55 MiB.

Revisão do código: a thread de armazenamento aguarda `recv()` em canal; o event
loop Slint recebe respostas por `upgrade_in_event_loop`. Não há polling nem
timer no aplicativo. O único timer adicionado está no teste headless, com prazo
máximo de 20 segundos para detectar falhas. O encerramento aguarda uma operação
pendente de banco; timeout de lock do SQLite é de três segundos.

## Arquivos criados ou alterados

| Arquivos | Finalidade |
| --- | --- |
| `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml` | Workspace, dependências resolvidas e toolchain |
| `.gitignore` | Ignorar artefatos de build e bancos locais |
| `crates/bobo-core/Cargo.toml`, `crates/bobo-core/src/lib.rs` | Entidades, IDs, erros, validação, comandos, contrato e testes de domínio |
| `crates/bobo-storage/Cargo.toml`, `crates/bobo-storage/src/lib.rs` | Adaptador SQLite e controle de migrações |
| `crates/bobo-storage/migrations/001_foundation.sql` | Schema inicial com integridade e preferências |
| `crates/bobo-storage/tests/persistence.rs` | Integração com banco temporário |
| `apps/desktop/Cargo.toml`, `apps/desktop/build.rs`, `apps/desktop/src/main.rs` | Binário e compilação Slint |
| `apps/desktop/src/application.rs`, `apps/desktop/src/worker.rs` | Controlador, seleção, rascunhos e I/O fora da UI |
| `apps/desktop/src/tests.rs` | Fluxo integrado dos callbacks Slint |
| `ui/app.slint`, `ui/components/nav-item.slint`, `ui/themes/tokens.slint` | Interface reativa, navegação reutilizável e temas |
| `.github/workflows/ci.yml` | Formatação, lint, testes e build em Windows/Ubuntu |
| `README.md`, `docs/VALIDATION.md` | Execução, decisões, resultados e limites |

O README já tinha mudanças do usuário: seu conteúdo de planejamento foi
preservado, com atualização de status e documentação da implementação.
O `AGENTS.md` fornecido pelo usuário foi lido integralmente e não foi alterado.
Não foram criados crates de protocolo/rede sem uso.

## Pendências e próxima ação

**Não verificado:** build/abertura em Linux, execução remota da CI, instalação
em Windows limpo, acessibilidade completa, DPI variado, recuperação de banco
corrompido, concorrência entre várias instâncias e desempenho sob carga.
Não foi realizada uma campanha manual completa de todas as operações por
mouse/teclado no Windows; o CRUD foi verificado pelos callbacks automatizados.
As dependências Linux e os comandos propostos estão no README.

O banco é local e não criptografado; histórico é carregado integralmente e não
há paginação. Seleção/rascunhos não persistem após fechar. Não há P2P, chamadas,
captura, nuvem, telemetria ou credenciais. A licença do projeto permanece a
definir; as alternativas de licença do Slint devem ser consideradas pelo
mantenedor antes de publicação.

Próxima ação sugerida: revisar a UI manualmente e executar os checks no Linux/CI.
Após aceitar a v0.1, iniciar o desenho do protocolo v0.2 e a prova de conexão real
entre dois dispositivos, sem integrar rede antes dessa validação.
