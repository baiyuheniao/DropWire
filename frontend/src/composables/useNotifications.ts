import { ref } from 'vue'

const permission = ref<NotificationPermission>('default')

function isSupported(): boolean {
  return typeof window !== 'undefined' && 'Notification' in window
}

export async function requestNotificationPermission(): Promise<NotificationPermission> {
  if (!isSupported()) return 'denied'
  if (permission.value === 'granted') return permission.value
  const result = await Notification.requestPermission()
  permission.value = result
  return result
}

export function notify(title: string, options?: NotificationOptions) {
  if (!isSupported()) return
  if (Notification.permission !== 'granted') return
  try {
    new Notification(title, {
      icon: '/favicon.ico',
      ...options,
    })
  } catch {
    // Ignore unsupported notification configurations in some browsers.
  }
}

export function useNotifications() {
  return {
    permission,
    requestPermission: requestNotificationPermission,
    notify,
  }
}
