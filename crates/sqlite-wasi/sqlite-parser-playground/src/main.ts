import './style.css'
import { initialize } from './visualize.ts'
import html_content from './assets/main.html?raw'

document.querySelector<HTMLDivElement>('#app')!.innerHTML = html_content

initialize(document.querySelector<HTMLDivElement>('#app')!)
