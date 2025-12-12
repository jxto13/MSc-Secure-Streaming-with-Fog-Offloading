Boa tarde a todos, o meu nome é Joaquim e vou vos apresentar a minha dissertação titulada "Secure Livestreaming with Fog Offloading".
Vou comecar por descrever que problema tenta resolver, referir outros trabalhos semelhantes e introduzir os seus requerimentos e objetivos.

No contexto de livestreaming usando Wifi, conforme maior a densidade de utilizadores na mesma infrastrutura, o performance começa a se degradar. 

Possiveis soluçoes para resolver este problema seria, aumentar a infrastrutra mas isso envolve um aumento de custo e nao escala. Usar multicast para poupar bandwith mas isso introduz perda de informaçao pela natureza do multicast sobre Wifi. Ou utilizar os recursos dos clientes para ajudar a infrastrutura.

Ou trabalhos utilizam partes destas soluçoes, referindo o Projecto IRIS desenvolvido pelo Professor Rolando e os seus colegas, e o Projeto Hyrax desenvolvido pelo INESC TEC.

FOG Computing, refere-se ao processo de atribuir tarefas a computaçao local retirando carga dos servidores cloud, diminuindo assim a latencia e requerimentos de bandwidth.

Os objetivos principais desta tese sao os seguintes, desenvolver e implementar um prototipo, desenvolver um protocolo de networking para integrar este prototipo, analisar e validar o desempenho do sistema, e verificar a sua usabilidade para adoçao.

Para tentar cumprir estes objetivos, os seguintes requerimentos foram postos:

- o sistema tem de autenticar e autorizar as sessioes dos clientes.

- assegurar que a informaçao se mantem protegida e inalterada ao longo do caminho.

- detetar informaçao perdida e recupera la quando possivel, assegurando sempre a sua ordem.

- centralizar as sessoes de clientes para gerir todas as politicas postas.

- suportar varios canais de comunicaçao e metodos

- existencia de metricas e informacao para resoluçao de problemas e analise

- assegurar que o sistema funciona sobre mas condiçoes

A linguagem escolhida para desenvolver este projeto foi Rust, devido a sua segurança de memoria em relaçao ao C++, e mantem um bom desempenho.

A arquitetura geral do sistema consiste em um encoder de video, que cria uma stream para ingetar no servidor, este depois de fazer o processamento necessario, envia para o acess point que depois entrega aos clientes usando Wifi.

No servidor o modulo Source Stream Handler, recebe os dados do encoder para processamento, guarda e aplica Reed-Solomon encoding em matrizes, isto cria columas e linhas de paridade que futuramente serve para recuperar informaçao, o aumento de colunas aumenta proteçao contra burst de perda enquanto linhas serao melhores para recuperaçao individual. Este numero de linhas e colunas pode ser estatico, ou seja, defenido na inicializaçao do servidor, ou dinamico com a ajuda de um Online Reinforcement Agent, neste projeto o algoritmo escolhido o Q-Learing  que usa uma tabela de estados e açoes para tomar decisoes, como esta tabela quanto maior pior a sua decisao, limitamos a reward function e estados possiveis usando um loss bin que agrupa intervalos relativamente a percentagem de erro reportada.

O membership manager identifica os clientes, recebo e processa os seus reports, esta informaçao depois sera utilizada no Network scheduler.

O network scheduler alterna a recuperaçao entre unicast e multicast, faz decisoes baseadas nos relatorios recebidos dos clientes, e controla a bandwitdh em multicast e por cliente. Para ajudar ainda a reduzir mais a bandwidth necessaria, é feito quando possivel o XOR de informaçao entre dois pacotes, permitindo assim quando um cliente so tem 1 dos 2, recuperar o outro.

Relativamente a parte do cliente, foi desenvolvido um Client Library, este que pode ser implementado em qualquer plataforma, um dos componentes é o decoder que recebe os pacotes, verifica e processa os mesmo, acabando por eventualmente mandar os reports para o servidor, fazendo assim de base para tudo o resto.

O retransmission manager é um middleware que fica entre a mesh network e o decoder, este recebe os pedidos da mesh e efetua a resposta necessaria, ele tem um cache sincronizado com o decoder, que sao independentes, evitando assim dead locks.

A tecnoogia usada para o offloading e implementaçao da mesh network é WIFI Aware, este é um hardware a parte do wifi, mas baseado na mesma tecnologia, tendo assim grande bandwidth e latencia baixa. Como é um hardware a parte nao interferece com a ligaçao wifi normal, podendo assim criar duas ligaçao simultanias.

Cada node na mesh networking, tem um ID, que faz broadcast pelo sockets ligados aos vizinhos que tem um TTL, assim existe uma maior possibilidade de recuparçao.

Um cliente android foi desenvolvido encapsular todos estes componentes, a interface disponibiliza o video ja processado, estaticas e qual ligaçoes peer to peer se encontram ativas.

Na parte da segurança, o uso do modelo PKI foi utilizado, cada servidor tem um CA que  todos os clientes confiam, e que cada um tem um certificado assinado por essa CA.

Como o uso de chaves publicas é muito grande, para encriptaçao da stream foi usado chaves simetricas que mudam apos um numero defenidos de epochs.


