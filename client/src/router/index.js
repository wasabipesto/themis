import { createRouter, createWebHistory } from 'vue-router'
import CalibrationView from '../views/CalibrationView.vue'
import AccuracyView from '../views/AccuracyView.vue'
import ListView from '../views/ListView.vue'
import AboutView from '../views/AboutView.vue'

const router = createRouter({
  history: createWebHistory(import.meta.env.BASE_URL),
  routes: [
    {
      path: '/calibration',
      alias: '/',
      name: 'calibration',
      component: CalibrationView
    },
    {
      path: '/accuracy',
      name: 'accuracy',
      component: AccuracyView
    },
    {
      path: '/about',
      name: 'about',
      component: AboutView
    },
    {
      path: '/list',
      name: 'list',
      component: ListView
    }
  ]
})

export default router
