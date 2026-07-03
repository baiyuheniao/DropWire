<template>
  <div class="send-view">
    <div class="send-options">
      <div class="option-group">
        <label>发送对象</label>
        <div class="receiver-field">
          <select v-model="receiverMode" class="receiver-select">
            <option value="">全局发送</option>
            <option v-for="u in knownUsers" :key="u.username" :value="u.username">
              {{ u.nickname || u.username }}
            </option>
            <option value="__custom__">指定用户...</option>
          </select>
          <input
            v-if="receiverMode === '__custom__'"
            v-model="customReceiver"
            type="text"
            class="custom-receiver"
            placeholder="输入接收者用户名"
            maxlength="32"
          />
        </div>
      </div>

      <div class="option-group">
        <label>备注</label>
        <input
          v-model="remark"
          type="text"
          class="remark-input"
          placeholder="可选：添加备注"
          maxlength="120"
        />
      </div>

      <div class="option-group">
        <label class="checkbox-label">
          <input v-model="enableEncryption" type="checkbox" />
          <span>加密发送（AES-256-GCM）</span>
        </label>
        <input
          v-if="enableEncryption"
          v-model="password"
          type="password"
          class="password-input"
          placeholder="输入加密密码"
          maxlength="64"
        />
        <p v-if="enableEncryption" class="encrypt-hint">
          加密后文件内容会经过浏览器本地加密再上传，接收方需要相同密码才能解密。
        </p>
      </div>
    </div>

    <FileUpload :options="uploadOptions" />
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import FileUpload from './FileUpload.vue'
import { type User } from './AccountModal.vue'

const props = defineProps<{
  user: User | null
}>()

const receiverMode = ref('')
const customReceiver = ref('')
const remark = ref('')
const enableEncryption = ref(false)
const password = ref('')
const knownUsers = ref<User[]>([])

onMounted(() => {
  const users: User[] = []
  for (let i = 0; i < localStorage.length; i++) {
    const key = localStorage.key(i)
    if (key?.startsWith('dropwire_user_')) {
      try {
        const data = JSON.parse(localStorage.getItem(key) || '{}')
        if (data.username) {
          users.push({
            username: data.username,
            nickname: data.nickname || data.username,
            avatar: data.avatar,
          })
        }
      } catch {
        // ignore invalid entries
      }
    }
  }
  knownUsers.value = users
})

const effectiveReceiver = computed(() => {
  if (receiverMode.value === '__custom__') return customReceiver.value.trim()
  return receiverMode.value
})

const uploadOptions = computed(() => ({
  sender: props.user?.nickname || props.user?.username || undefined,
  receiver: effectiveReceiver.value || undefined,
  remark: remark.value.trim() || undefined,
  password: enableEncryption.value ? password.value : undefined,
}))
</script>

<style scoped>
.send-view {
  max-width: 720px;
  margin: 0 auto;
}

.send-options {
  display: grid;
  gap: 16px;
  margin-bottom: 20px;
  padding: 18px 20px;
  background: #fff;
  border: 1px solid #e5e7eb;
  border-radius: 16px;
}

.option-group label {
  display: block;
  margin-bottom: 8px;
  font-size: 14px;
  font-weight: 500;
  color: #374151;
}

.receiver-field {
  display: flex;
  gap: 12px;
  align-items: center;
}

.receiver-select,
.custom-receiver,
.remark-input {
  padding: 10px 12px;
  border: 1px solid #d1d5db;
  border-radius: 10px;
  font-size: 14px;
  outline: none;
  transition: border-color 0.2s, box-shadow 0.2s;
}

.receiver-select:focus,
.custom-receiver:focus,
.remark-input:focus {
  border-color: #3b82f6;
  box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.1);
}

.receiver-select {
  min-width: 160px;
  background: #fff;
}

.custom-receiver,
.remark-input {
  flex: 1;
  min-width: 0;
}

.checkbox-label {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  font-size: 14px;
  font-weight: 500;
  color: #374151;
  cursor: pointer;
  margin-bottom: 10px;
}

.checkbox-label input {
  width: 16px;
  height: 16px;
  accent-color: #3b82f6;
}

.password-input {
  width: 100%;
  padding: 10px 12px;
  border: 1px solid #d1d5db;
  border-radius: 10px;
  font-size: 14px;
  outline: none;
  transition: border-color 0.2s, box-shadow 0.2s;
}

.password-input:focus {
  border-color: #3b82f6;
  box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.1);
}

.encrypt-hint {
  margin-top: 8px;
  font-size: 12px;
  color: #6b7280;
  line-height: 1.5;
}
</style>
