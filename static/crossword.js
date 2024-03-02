class CrosswordGrid extends HTMLElement {

    constructor() {
        super();
        const shadowRoot = this.attachShadow({ mode: 'closed' })

        this.grid = document.createElement('div')
        this.grid.id = "crossword"
        this.grid.style.background = "black"
        this.grid.style.width = 0
        shadowRoot.append(this.grid)

        this.acrossHintsParent = document.createElement('div')
        this.acrossHintsParent.id = "across-hint-container"
        this.acrossHintsParent.textContent = "Across clues"
        shadowRoot.append(this.acrossHintsParent)
        this.acrossHints = document.createElement('ul')
        this.acrossHints.id = "across-hints"
        this.acrossHintsParent.appendChild(this.acrossHints)

        this.downHintsParent = document.createElement('div')
        this.downHintsParent.id = "down-hint-container"
        this.downHintsParent.textContent = "Down clues"
        shadowRoot.append(this.downHintsParent)
        this.downHints = document.createElement('ul')
        this.downHints.id = "down-hints"
        this.downHintsParent.appendChild(this.downHints)

        this.data = null;

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
            console.log("Message from server ",message);            
        });

        this.fetchData().then(() => {
        });
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
                this.drawFreshGrid()
                this.grid.tabIndex = 0
                this.grid.focus()

                this.grid.addEventListener('keydown', (key) => {
                    if (this.activeClue===null) {

                    } else {
                        this.activeClue.highlight()
                        // console.log(key)
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
            this.handleIncomingClue(incomingClueName, this.acrossHints, incomingClueData);
        }

        for (let incomingClueName in this.data.down) {
            let incomingClueData = this.data.down[incomingClueName];
            this.handleIncomingClue(incomingClueName, this.downHints, incomingClueData);
        }

    }

    handleUpdateTextFromServer(new_cell) {
        console.log(new_cell)
        let key = `${new_cell.x},${new_cell.y}`
        console.log(key)
        // let frobnicate = new Cell(new_cell, this.scale)
        let cell = this.cells.get(key)
        cell.text = new_cell.c
        cell.updateText(new_cell.c)
        // this.cells.set(key, frobnicate)
        console.log(this.cells)

        // this.grid.replaceChild(frobnicate.div, old_div.div)
    }

    handleIncomingClue(incomingClueName, clueDirection, incomingClueData) {
        let hintEl = this.createHintElement(incomingClueName, incomingClueData);
        clueDirection.appendChild(hintEl);

        let clue = new Clue(incomingClueName);

        for (let incomingCellData in incomingClueData.cells) {
            let cellData = incomingClueData.cells[incomingCellData];
            let key = `${cellData.x},${cellData.y}`;
            if (!this.cells.has(key)) {
                let cell = new Cell(cellData, this.scale);
                cell.div.addEventListener('click', () => {
                    var childNodes = this.grid.childNodes;
                    childNodes.forEach(node => {
                        node.style.background = "#ffffff66";
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
        let hintEl = document.createElement('li');
        hintEl.style.listStyleType = "none";
        hintEl.textContent = `${clueName}) ${clueData["hint"]}`;
        return hintEl;
    }

    expandBackgroundElement(cellData) {
        if ((cellData.x + 1) * this.scale > this.grid.clientWidth) {
            this.grid.style.width = (cellData.x + 1) * this.scale + "px";
        }
        if ((cellData.y + 1) * this.scale > this.grid.clientHeight) {
            this.grid.style.height = (cellData.y + 1) * this.scale + "px";
        }
    }

}

class Cell {
    constructor(cellData, scale) {
        console.log("creating cell")
        let div = document.createElement('div');
        div.style.position = 'absolute';
        div.style.top = cellData.x * scale + 'px';
        div.style.left = cellData.y * scale + 'px';
        div.style.width = scale + 'px';
        div.style.height = scale + 'px';
        div.style.background = "#ffffff66";
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
        this.div.style.background = "green"
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
                this.cells[this.cellIdx].div.style.background = "red"
                console.log(`active cell ${this.cells[this.cellIdx].coords}`)
                return
            }
        }
        this.cellIdx = this.cells.length - 1
        this.cells[this.cellIdx].div.style.background = "red"
        console.log(`active cell ${this.cells[this.cellIdx].coords}`)
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