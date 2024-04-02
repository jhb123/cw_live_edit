class CrosswordGrid extends HTMLElement {

    constructor() {
        super();
        const shadowRoot = this.attachShadow({ mode: 'closed' })

        fetch(`/crossword.html`)
        .then(response => {
            if (!response.ok) {
                throw new Error("Failed to get crossword data")
            }
            return response.text();
        })
        .then( template_data => {

            const template = document.createElement('template');
            
            template.innerHTML = template_data;
            shadowRoot.appendChild(template.content.cloneNode(true));
            this.grid = shadowRoot.getElementById('crossword')

            this.invisible_input = document.createElement('input');
            this.invisible_input.style.display = "none"
            // this.grid.contentEditable = true;

            this.acrossHintsParent = shadowRoot.getElementById('across-hint-container')
            this.acrossHints = shadowRoot.getElementById('across-hints')

            this.downHintsParent = shadowRoot.getElementById('down-hint-container')
            this.downHints = shadowRoot.getElementById('down-hints')

            this.data = null;

            this.downHintsData = []
            this.acrossHintsData = []

            this.scale = 30
            this.cells = new Map();
            this.activeClue = null;
            
            this.src = this.getAttribute('src') || ''

            let loc = window.location.host + this.src

            this.ws = new WebSocket("ws://" + loc + '/live')
    
            // Connection opened
            this.ws.addEventListener("open", (event) => {
            this.ws.send("Hello Server!");
            });
            
            // Listen for messages
            this.ws.addEventListener("message", (event) => {
                let message = JSON.parse(event.data);
                this.handleUpdateTextFromServer(message)
            });

            this.fetchData().then(() => {
            });
        })
    }

    async fetchData() {
        fetch(`${this.src}/data`)
            .then(response => {
                if (!response.ok) {
                    throw new Error("Failed to get crossword data")
                }
                return response.json();
            })
            .then(data => {
                this.data = data
                let size = 0
                for (let key in this.data["across"]) {
                // this.data.across.forEach( clue => {
                    let clue = this.data["across"][key]
                    clue.cells.forEach( coord => {
                        if (coord.x > size){
                            size = coord.x
                        }
                        if (coord.x > size){
                            size = coord.y
                        }
                    })
                }
                this.scale = 100/(size+1)

                this.drawFreshGrid()
                this.grid.tabIndex = 0
                this.invisible_input.focus()

                this.grid.addEventListener('keydown', (key) => {
                    if (this.activeClue===null) {

                    } else {
                        this.activeClue.highlight()
                        let cell;
                        switch(key.key) {
                            case "Backspace":
                                this.activeClue.getActiveCell().updateText(" ");
                                this.ws.send(this.activeClue.getActiveCell().getCellData())
                                cell = this.activeClue.backwardCellIterator.next().value;
                                this.activeClue.setActiveCell(cell);
                                break;
                            case "ArrowRight":
                            case 'ArrowDown':
                                cell = this.activeClue.forwardCellIterator.next().value;
                                this.activeClue.setActiveCell(cell);
                                break;
                            case "ArrowLeft":
                            case 'ArrowUp':
                                cell = this.activeClue.backwardCellIterator.next().value;
                                this.activeClue.setActiveCell(cell);
                                break;
                            default:
                                if (/^[a-zA-Z]$/.test(key.key)) {
                                    this.activeClue.getActiveCell().updateText(key.key);
                                    this.ws.send(this.activeClue.getActiveCell().getCellData())
                                    cell = this.activeClue.forwardCellIterator.next().value;
                                    this.activeClue.setActiveCell(cell);
                                }
                                else {
                                    cell = this.activeClue.getActiveCell()
                                    this.activeClue.setActiveCell(cell);

                                }
                        }
                    }
                })
                // this.grid.addEventListener('click', () => {
                //     console.log("grid clicked")
                // })

            })
    }


    drawFreshGrid() {
        for (let incomingClueName in this.data.across) {
            let incomingClueData = this.data.across[incomingClueName];
            this.handleIncomingClue(incomingClueName, this.acrossHintsData, incomingClueData);
        }

        for (let incomingClueName in this.data.down) {
            let incomingClueData = this.data.down[incomingClueName];
            this.handleIncomingClue(incomingClueName, this.downHintsData, incomingClueData);
        }

        this.drawHints()

    }

    handleUpdateTextFromServer(new_cell) {
        let key = `${new_cell.x},${new_cell.y}`
        let cell = this.cells.get(key)
        cell.text = new_cell.c
        cell.updateText(new_cell.c)
    }

    sortfn(a,b) {
        const numA = parseInt(a.name);
        const numB = parseInt(b.name);        
        if (numA < numB) {
            return -1;
        } else if (numA > numB) {
            return 1;
        } else {
            return a.name.localeCompare(b.name);
        }    
    }

    drawHints(){
        this.acrossHintsData.sort(this.sortfn)
        this.downHintsData.sort(this.sortfn)
        this.acrossHintsData.forEach( clue => {
            let hintEl = this.createHintElement(clue.name, clue.value);
            this.acrossHints.appendChild(hintEl);
        });

        this.downHintsData.forEach( clue => {
            let hintEl = this.createHintElement(clue.name, clue.value);
            this.downHints.appendChild(hintEl);
        });

    }

    handleIncomingClue(incomingClueName, clueDirection, incomingClueData) {
        // let hintEl = this.createHintElement(incomingClueName, incomingClueData);
        clueDirection.push({name: incomingClueName, value: incomingClueData["hint"]});

        let clue = new Clue(incomingClueName);

        for (let incomingCellData in incomingClueData.cells) {
            let cellData = incomingClueData.cells[incomingCellData];
            let key = `${cellData.x},${cellData.y}`;
            if (!this.cells.has(key)) {
                let cell = new Cell(cellData, this.scale);
                cell.div.addEventListener('click', () => {
                    var childNodes = this.grid.childNodes;
                    childNodes.forEach(node => {
                        node.style.background = "#ffffffff";
                    });
                    this.activeClue = cell.handleClick();
                    this.activeClue.setActiveCell(cell)
                });

                this.cells.set(key, cell);
            }
            let cellClass = this.cells.get(key);

            clue.cells.push(cellClass);
            cellClass.cluesPartof.push(clue);



            this.grid.append(cellClass.div);
            this.expandBackgroundElement(cellData);
        }
    }

    createHintElement(clueName, clueData) {
        let hintEl = document.createElement('tr');
        hintEl.innerHTML =
        `<td class="clue-hint-num">${clueName}</td>
        <td class="clue-hint-text">${clueData}</td>`
        hintEl.classList.add("clue-hint");
        return hintEl;
    }

    expandBackgroundElement(cellData) {
        this.grid.style.height = this.grid.width
        // if ((cellData.x + 1) * this.scale > this.grid.clientWidth) {
        //     this.grid.style.width = (cellData.x + 1) * this.scale + "px";
        // }
        // if ((cellData.y + 1) * this.scale > this.grid.clientHeight) {
        //     this.grid.style.height = (cellData.y + 1) * this.scale + "px";
        // }
    }

}

class Cell {
    constructor(cellData, scale) {
        let div = document.createElement('div');
        // div.tabIndex=0
        // div.contentEditable = true;
        div.style.position = 'absolute';
        div.style.left = cellData.x * scale + '%';
        div.style.top = cellData.y * scale + '%';
        div.style.width = scale + '%';
        div.style.height = scale + '%';
        div.style.background = "#ffffffff";
        div.style.boxSizing = "border-box";
        div.style.border = '1px solid black';
        div.style.textAlign = "center"
        div.style.verticalAlign = "middle"
        
        this.text = ""
        this.div = div;
        this.cluesPartof = []
        this.clueIterator = this.cycleClue()
        this.coords = cellData

        this.updateText(cellData.c)


    }

    getCellData(){
        return JSON.stringify({x: this.coords.x, y: this.coords.y, c: this.text })
    }

    handleClick() {
        let clue = this.clueIterator.next().value
        if (clue === null) {
            console.warn("cell not part of a clue")
        } else {
            clue.highlight()
            return clue
        }
    }

    updateText(text) {
        this.text = text
        this.div.textContent = this.text 

    }

    *cycleClue() {
        var index = 0;
        while (true) {
            if (this.cluesPartof.length === 0 ) {  
                console.warn("cell is not part of any clue")
                yield null;    
            }
            else {
                yield this.cluesPartof[index]
                index++
                if (index === this.cluesPartof.length){
                    index = 0;
                }
            }
        }
    }

    handleHighlight() {
        this.div.style.background = "#B6FFDA"
    }




}

class Clue {
    constructor(clueName) {
        this.clueName = clueName
        this.cells = []
        this.hint = ""
        this.forwardCellIterator = this.moveCellForward()
        this.backwardCellIterator = this.moveCellBackward()
        this.cellIdx = null
    }

    highlight() {
        this.cells.forEach( cell => {
            cell.handleHighlight()
        })
    }

    getActiveCell() {
        return this.cells[this.cellIdx]
    }

    setActiveCell(cell) {
        for(var i = 0; i < this.cells.length; ++i)  {
            if (this.cells[i] === cell) {
                this.cellIdx = i
                this.cells[this.cellIdx].div.style.background = "#FFF8B6"
                return
            }
        }
        this.cellIdx = this.cells.length - 1
        this.cells[this.cellIdx].div.style.background = "#FFF8B6"
    }

    *moveCellForward() {
        while (true) {
            if (this.cellIdx === this.cells.length){
                yield this.cells[this.cellIdx]
            } else {
                this.cellIdx++
                yield this.cells[this.cellIdx]
            }                
        }
    }

    *moveCellBackward() {
        while (true) {
            if (this.cellIdx === 0){
                yield this.cells[this.cellIdx]
            } else {
                this.cellIdx--
                yield this.cells[this.cellIdx]
            }                
        }
    }

}

window.customElements.define("crossword-grid", CrosswordGrid)