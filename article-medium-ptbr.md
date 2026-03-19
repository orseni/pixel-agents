# Pixel Agents Standalone: Monitorando Todas as Sessões do Claude Code como Personagens Pixel Art

Tem algo estranhamente satisfatório em assistir seus agentes de IA trabalhando. Não o output do terminal passando na tela, mas realmente *ver* eles como personagens sentados em mesas, digitando, andando por um escritório.

Essa é toda a ideia por trás do Pixel Agents. Ele transforma suas sessões do Claude Code em personagens pixel art animados vivendo num pequeno escritório. É divertido. Fica bonito. E quando você coloca pra rodar, vai se pegar olhando pra ele bem mais do que esperava.

![SCREENSHOT: O app standalone Pixel Agents mostrando o escritório pixel art com múltiplos personagens nas mesas, cada um com o nome do seu projeto. Alguns personagens digitando, um idle. O app roda em sua própria janela. Legenda: "Cada sessão ativa do Claude Code na máquina, visualizada como personagens pixel art em um escritório compartilhado."]

## O Projeto Original

[Pixel Agents](https://github.com/pablodelucca/pixel-agents) é uma extensão do VS Code criada pelo Pablo de Lucca. O conceito é simples e, sinceramente, genial: cada terminal do Claude Code que você abre ganha seu próprio personagem animado em um pequeno escritório pixel art. O personagem caminha até uma mesa, senta, e começa a digitar quando o agente está escrevendo código. Ele lê quando o agente está buscando arquivos. Mostra um balão de fala quando está esperando seu input.

Não é só enfeite. Depois de usar por um tempo, você percebe que tem utilidade real ali. Dá pra bater o olho no escritório e saber instantaneamente o estado de cada agente. Sem precisar clicar em abas de terminal. O feedback visual é imediato.

O projeto também vem com um editor de layout completo. Dá pra redesenhar o escritório, colocar móveis, pintar pisos e paredes, e exportar o layout como JSON. Tem notificações sonoras, visualização de sub-agentes, sprites de personagens diversos. É um software muito bem feito.

## Levando Adiante: um App Standalone

A extensão original funciona muito bem dentro do VS Code. Mas me fez pensar num ângulo diferente. E se esse mesmo conceito pudesse rodar como um app standalone, monitorando *todas* as sessões do Claude Code na máquina de uma vez? Não só as lançadas pelo VS Code, mas sessões rodando no iTerm, Warp, Cursor, múltiplas janelas do VS Code, qualquer lugar.

Essa ideia virou este fork. Construído em cima do Pixel Agents original, esta versão substitui o backend do VS Code por um app desktop standalone usando [Tauri](https://tauri.app/) e Rust. O frontend (o escritório pixel art, o editor de layout, todas as animações) permanece praticamente intocado. O que mudou é como o app descobre e monitora as sessões do Claude Code.

**[GitHub: orseni/pixel-agents](https://github.com/orseni/pixel-agents)**

Ao invés de gerenciar terminais do VS Code, o app monitora `~/.claude/projects/` diretamente. Esse é o diretório onde o Claude Code armazena seus arquivos de transcrição JSONL, independente de qual terminal ou editor o lançou.

A cada cinco segundos, o backend Rust escaneia por arquivos JSONL recentemente ativos. Quando encontra um, cria um agente, começa a monitorar o arquivo por mudanças, e emite eventos pro frontend. Um novo personagem entra no escritório, senta numa mesa, e começa a fazer o que quer que a sessão real do Claude Code esteja fazendo.

Quando uma sessão fica ociosa, o personagem eventualmente desaparece.

Sem precisar do VS Code. Sem setup manual. É só abrir o app e trabalhar com o Claude Code do jeito que você já faz.

## Como Funciona por Dentro

O backend Rust faz o que o backend original em Node.js/VS Code fazia, mas sem dependência de editor:

**Descoberta de sessões** escaneia `~/.claude/projects/` por arquivos JSONL modificados recentemente. O Claude Code organiza transcrições por projeto, usando uma versão hasheada do caminho do diretório como nome da pasta. O backend reconstrói o nome original do projeto a partir desse hash usando um algoritmo guloso de resolução de caminhos que verifica quais diretórios realmente existem no disco.

**Monitoramento de arquivos** usa polling (intervalo de 1 segundo) para ler novas linhas de cada arquivo JSONL incrementalmente, com buffer de linhas parciais para leituras durante escrita. A mesma abordagem testada em batalha da extensão original, que aprendeu na prática que `fs.watch` não é confiável no macOS.

**Parsing de transcrições** lida com todos os mesmos tipos de registro JSONL: mensagens `assistant` com blocos tool_use, mensagens `user` com tool_results, registros `system` com sinais de turn_duration, e registros `progress` para atividade de sub-agentes. Cada ferramenta é formatada em um status legível ("Reading config.ts", "Running: npm test", "Searching code").

**Gerenciamento de timers** detecta quando um agente pode estar travado esperando permissão (timeout de 7 segundos) ou finalizou uma resposta apenas texto (detecção de inatividade de 5 segundos). São as mesmas heurísticas que o original usa.

**Bridge de IPC** é onde fica interessante. O frontend foi construído pra receber `MessageEvent`s da API `postMessage` do VS Code. Ao invés de reescrever tudo isso, o adaptador Tauri escuta eventos do backend Rust e dispara `MessageEvent`s sintéticos no objeto window. Os React hooks que gerenciam o estado dos agentes não sabem e não se importam se os eventos vêm do Rust ou do VS Code. Zero mudanças na lógica core da UI.

## O Que Muda (e o Que Continua Igual)

O editor de layout funciona exatamente como o original. Pisos, paredes, móveis, undo/redo, exportar/importar. Seu `~/.pixel-agents/layout.json` é compartilhado, então se você usava a extensão do VS Code antes, seu layout de escritório é mantido.

As animações dos personagens são idênticas. Mesmos sprites, mesma máquina de estados (idle, walk, type, read), mesmo pathfinding, mesmos efeitos estilo matrix de spawn/despawn.

O que é novo:

**Descoberta automática.** Você nunca clica em "+ Agent". Personagens aparecem e desaparecem baseado no que está realmente rodando na sua máquina.

**Labels de projeto.** Cada personagem mostra o nome do projeto em que está trabalhando. Quando você tem agentes em múltiplos projetos, isso é o que torna a visão do escritório realmente útil.

**Um app pra tudo.** Ao invés de uma extensão por janela do VS Code (cada uma mostrando apenas seus próprios terminais), você tem um app mostrando todas as sessões de qualquer lugar.

**Backend em Rust.** O monitoramento de arquivos, parsing de JSONL e gerenciamento de timers rodam em Rust assíncrono com Tokio. É leve e o binário tem cerca de 13 MB.

## Como Rodar

Você vai precisar do Rust e Node.js instalados. Depois:

```bash
git clone https://github.com/orseni/pixel-agents.git
cd pixel-agents
npm install
cd webview-ui && npm install && cd ..
npm run dev    # desenvolvimento com hot reload
npm run build  # binário de produção
```

O binário de produção fica em `src-tauri/target/release/pixel-agents`. Dá pra copiar pra qualquer lugar e executar. O frontend é embutido no binário.

Pro uso do dia a dia, é só lançar o binário e deixar aberto. Abra o Claude Code em qualquer terminal, qualquer projeto, qualquer editor. Os personagens aparecem sozinhos.

## Créditos

Todo o crédito pelo conceito original, o engine pixel art, o editor de layout e o sistema de personagens vai pro [Pablo de Lucca e os contribuidores do Pixel Agents](https://github.com/pablodelucca/pixel-agents). Os sprites dos personagens são baseados no trabalho do [JIK-A-4, Metro City](https://jik-a-4.itch.io/metrocity-free-topdown-character-pack). Este fork apenas pega o excelente trabalho deles e coloca num pacote standalone que consegue monitorar toda a máquina ao invés de uma única janela de editor.

Se você prefere a integração com VS Code, a extensão original é ótima e está disponível no [VS Code Marketplace](https://marketplace.visualstudio.com/items?itemName=pablodelucca.pixel-agents).

---

*O fork é open source sob a licença MIT. Issues e contribuições são bem-vindas no [GitHub](https://github.com/orseni/pixel-agents).*
