# SIGMAP

**Sigma P** é um simulador de execução de processos de Sistemas Operacionais

## Licença 

Software Livre, com fins educacionais
Licença MIT

## Roadmap

- [ ] Escalonador
- [x] PID
- [x] Criação de novos processos
- [x] Alocação de memória
- [x] Bloqueio de tempo
- [ ] Bloqueio de chamada de sistema
- [x] Processos CPU Bounding
- [x] 1 núcleo de CPU
- [ ] 2 núcleos de CPU
- [ ] até 4 núcleos de CPU
- [ ] Interrupções
- [ ] Processos Io Bouding
- [ ] Processos CPU/Io Bounding
- [ ] Periféricos
- [ ] Salvar a simulação no banco de dados

## Como Instalar

Acesse os binários pré-compilados para sua plataforma

```
bin/
  win/
  lin/
  mac/
```

## Como citar 
VITOR et al, Sigma P - Simulador de processos SO. Versão 0.1. UNIFAGOC, 2026.

## Contribua com o projeto 

Compile para novas arquiteturas e faça um PR disponibilizando o binário na pasta bin
Ou faça um fork e customize de acordo com o seu objetivo

## Construindo a partir do fonte
Instale o Rust, clone e build

### Windows
Instale o Rust,
Instale as ferramentas do Microsoft Visual Studio Builder
Marque a opção C++ Desktop essential

### Linux

Instala o rust e configura a variável de ambientes
Confirme se o gcc está configurado

```rust
sudo apt install curl -y
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
export PATH="$HOME/.cargo/bin:$PATH"

apt install build-essential
```

## Clone do projeto

```rust
sudo apt install git -y
git clone https://github.com/ricardodarocha/sigmap 
cd sigmap

cargo run 
```
