<template>
  <div class="modal-overlay" @click.self="emit('close')">
    <div class="modal">
      <div class="modal-header">
        <h3>{{ title }}</h3>
        <button class="close-btn" @click="emit('close')">&times;</button>
      </div>

      <div class="modal-body">
        <!-- 已登录：编辑账户 -->
        <template v-if="isLoggedIn">
          <div class="form-group">
            <label>昵称</label>
            <input v-model="editForm.nickname" type="text" placeholder="输入昵称" />
          </div>
          <div class="form-group">
            <label>头像 URL</label>
            <input v-model="editForm.avatar" type="text" placeholder="留空使用默认头像" />
          </div>
          <button class="submit-btn" @click="saveProfile">保存修改</button>
          <button class="link-btn" @click="logout">退出登录</button>
        </template>

        <!-- 未登录：登录 / 注册 -->
        <template v-else>
          <div class="form-group">
            <label>用户名</label>
            <input v-model="authForm.username" type="text" placeholder="输入用户名" />
          </div>
          <div class="form-group">
            <label>密码</label>
            <input v-model="authForm.password" type="password" placeholder="输入密码" />
          </div>
          <div v-if="mode === 'register'" class="form-group">
            <label>确认密码</label>
            <input v-model="authForm.confirmPassword" type="password" placeholder="再次输入密码" />
          </div>
          <div v-if="error" class="error">{{ error }}</div>
          <button class="submit-btn" @click="handleAuth">
            {{ mode === 'login' ? '登录' : '注册' }}
          </button>
          <button class="link-btn" @click="toggleMode">
            {{ mode === 'login' ? '没有账户？去注册' : '已有账户？去登录' }}
          </button>
        </template>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import axios from 'axios'
import { settings } from '../composables/useSettings'

export interface User {
  username: string
  nickname: string
  avatar?: string
}

const props = withDefaults(defineProps<{
  modelValue?: boolean
  user: User | null
}>(), {
  modelValue: false,
})

const emit = defineEmits<{
  (e: 'close'): void
  (e: 'update:user', user: User | null): void
}>()

const isLoggedIn = computed(() => !!props.user)
const mode = ref<'login' | 'register'>('login')
const error = ref('')
const loading = ref(false)

const title = computed(() => {
  if (isLoggedIn.value) return '编辑账户'
  return mode.value === 'login' ? '登录' : '注册'
})

const authForm = ref({
  username: '',
  password: '',
  confirmPassword: '',
})

const editForm = ref({
  nickname: props.user?.nickname || '',
  avatar: props.user?.avatar || '',
})

watch(
  () => props.user,
  (user) => {
    editForm.value = {
      nickname: user?.nickname || '',
      avatar: user?.avatar || '',
    }
  },
  { immediate: true }
)

function apiUrl(path: string): string {
  const base = settings.value.apiBase.trim()
  if (!base) return path
  return `${base.replace(/\/$/, '')}${path}`
}

function persistUser(user: User | null) {
  if (user) {
    localStorage.setItem('dropwire_current_user', JSON.stringify(user))
  } else {
    localStorage.removeItem('dropwire_current_user')
  }
}

function toggleMode() {
  mode.value = mode.value === 'login' ? 'register' : 'login'
  error.value = ''
}

async function handleAuth() {
  error.value = ''
  const { username, password, confirmPassword } = authForm.value
  if (!username || !password) {
    error.value = '请填写用户名和密码'
    return
  }
  if (mode.value === 'register' && password !== confirmPassword) {
    error.value = '两次输入的密码不一致'
    return
  }

  loading.value = true
  try {
    const endpoint = mode.value === 'login' ? '/auth/login' : '/auth/register'
    const res = await axios.post<{ success: boolean; message: string; data?: User }>(
      apiUrl(endpoint),
      { username, password, nickname: username, avatar: '' },
    )
    if (!res.data.success) {
      error.value = res.data.message || '请求失败'
      return
    }
    const user = res.data.data!
    persistUser(user)
    emit('update:user', user)
    authForm.value = { username: '', password: '', confirmPassword: '' }
  } catch (err: any) {
    error.value = err?.response?.data?.message || '网络错误，请稍后重试'
  } finally {
    loading.value = false
  }
}

async function saveProfile() {
  if (!props.user) return
  const { nickname, avatar } = editForm.value
  loading.value = true
  try {
    const res = await axios.post<{ success: boolean; message: string; data?: User }>(
      apiUrl('/auth/profile'),
      { username: props.user.username, nickname, avatar },
    )
    if (!res.data.success) {
      error.value = res.data.message || '保存失败'
      return
    }
    const user = res.data.data!
    persistUser(user)
    emit('update:user', user)
  } catch (err: any) {
    error.value = err?.response?.data?.message || '网络错误'
  } finally {
    loading.value = false
  }
}

function logout() {
  persistUser(null)
  emit('update:user', null)
}
</script>

<style scoped>
.modal-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.45);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.modal {
  width: 380px;
  background: var(--bg-card);
  border-radius: 16px;
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.25);
  overflow: hidden;
}

.modal-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 18px 20px;
  border-bottom: 1px solid var(--border-color);
}

.modal-header h3 {
  font-size: 18px;
  font-weight: 600;
  color: var(--text-primary);
}

.close-btn {
  background: none;
  border: none;
  font-size: 24px;
  color: var(--text-tertiary);
  cursor: pointer;
  line-height: 1;
}

.close-btn:hover {
  color: var(--text-secondary);
}

.modal-body {
  padding: 20px;
}

.form-group {
  margin-bottom: 16px;
}

.form-group label {
  display: block;
  margin-bottom: 6px;
  font-size: 14px;
  font-weight: 500;
  color: var(--text-secondary);
}

.form-group input {
  width: 100%;
  padding: 10px 12px;
  border: 1px solid var(--border-strong);
  border-radius: 8px;
  font-size: 14px;
  background: var(--bg-input);
  color: var(--text-primary);
  outline: none;
  transition: border-color 0.2s;
}

.form-group input:focus {
  border-color: var(--primary);
}

.error {
  margin-bottom: 12px;
  color: var(--danger-text);
  font-size: 13px;
}

.submit-btn {
  width: 100%;
  padding: 11px;
  background: var(--primary);
  color: #fff;
  border: none;
  border-radius: 8px;
  font-size: 15px;
  font-weight: 500;
  cursor: pointer;
  transition: background 0.2s;
}

.submit-btn:hover {
  background: var(--primary-hover);
}

.link-btn {
  width: 100%;
  margin-top: 12px;
  padding: 8px;
  background: none;
  border: none;
  color: var(--primary);
  font-size: 14px;
  cursor: pointer;
}

.link-btn:hover {
  text-decoration: underline;
}
</style>
