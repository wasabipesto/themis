import { createRouter, createWebHistory } from 'vue-router'
import IntroductionView from '../views/IntroductionView.vue'
import CalibrationView from '../views/CalibrationView.vue'
import AccuracyView from '../views/AccuracyView.vue'
import ListView from '../views/ListView.vue'
import FAQView from '../views/FAQView.vue'
import NotFoundView from '../views/NotFoundView.vue'

const router = createRouter({
  history: createWebHistory(import.meta.env.BASE_URL),
  routes: [
    {
      path: '/introduction',
      name: 'introduction',
      component: IntroductionView
    },
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
      path: '/list',
      name: 'list',
      component: ListView
    },
    {
      path: '/faq',
      name: 'faq',
      component: FAQView
    },
    {
      path: '/:pathMatch(.*)*', // this is a wildcard route
      name: 'not-found',
      component: NotFoundView
    }
  ]
})

export default router
