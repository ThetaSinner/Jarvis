import { createApp } from 'vue'
import App from './App.vue'
import router from './router'

// Type errors on the provided code?
// @ts-ignore
createApp(App).use(router).mount('#app')
