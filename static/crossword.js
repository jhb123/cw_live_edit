class CrosswordGrid extends HTMLElement {
    constructor(){
        super();
        const shadowRoot = this.attachShadow({mode: 'closed'})
        let div = document.createElement('div')
        div.textContent = "Crossword grid"
        shadowRoot.append(div)
    }
}

window.customElements.define("crossword-grid", CrosswordGrid)