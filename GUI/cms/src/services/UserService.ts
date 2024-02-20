import { URL } from "./constents";

export interface User {
    userId: number
}

export class UserService {
    static getUserFromJson(json: any): User {
        return { userId: json['user_id'] }
    }
    static async login(username: string, password: string): Promise<User> {
        return await fetch(`${URL}/login?username=${username}&password=${password}`, {
            method: 'POST',

        }).then((response) => response.json()).then((json) => {
            console.log(json)
            return this.getUserFromJson(json)
        });
    }
    static async signup(username:string,password:string,permissions:object):Promise<User>{
        return await fetch(`${URL}/register`,{
            method:'POST',
            headers: {
                'Content-Type': 'application/json',
            },

            body:JSON.stringify({
                'permissions':permissions,
                'username':username,
                'password':password
            })
        })
        .then((response)=>response.json())
        .then((json)=>this.getUserFromJson(json))
    }
    static async logout(userId:number){
        await fetch(`${URL}/logout?user_id=${userId}`)
    }
    static async logoutWithUser(user:User){
        await this.logout(user.userId)
    }
}