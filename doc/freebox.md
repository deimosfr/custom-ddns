# Freebox

## Authorise the app

```bash
$ curl -X POST -H 'Content-type: application/json' http://mafreebox.freebox.fr/api/v13/login/authorize/ -d '{"app_id": "fr.freebox.cddns","app_name": "Cusstom DDNS","app_version": "1.0.0","device_name": "Freebox"}'
{"success":true,"result":{"app_token":"token","track_id":1}}
```

Replace the elements in the `app_id`, `app_name` and `device_name` with the values you want to use.

Once done, you have to authorize on the Freebox the application request. You can then login on the Freebox to remove all application permissions in the access management.

You can then check the application is allowed to access the Freebox API (1 is corresponding to the track_id from the previous request): 

```bash
$ export app_token=<app_token given above>
$ curl -H 'Content-type: application/json' http://mafreebox.freebox.fr/api/v13/login/authorize/1
{"success":true,"result":{"status":"granted","challenge":"xxx","password_salt":"yyy"}}
```

## Login

```bash
$ export challenge=<challenge given above>
$ export password_salt=<password_salt given above>
$ export password=$(echo -n "$challenge" | openssl sha1 -hmac "$app_token" | cut -d '=' -f2 | sed 's/ //g')
$ curl -X POST -H 'Content-type: application/json' http://mafreebox.freebox.fr/api/v13/login/ -d "{\"app_id\": \"fr.freebox.cddns\",\"password\": \"$password\"}"
```

## Open a session

```bash
curl -X POST -H 'Content-type: application/json' http://mafreebox.freebox.fr/api/v8/session/ -d '{"app_id": "fr.freebox.cddns","password": "yyy"}'
```



## Configuration example

```yaml
dns_records:
  - name: "home"
    source:
      freebox:
        url: "http://mafreebox.freebox.fr"
        token: "1234567890"
      check_interval_in_seconds: 20
    domain:
      provider: "cloudflare"
      domain_name: "xxx.com"
      record_name: "yyy"
      record_type: "CNAME"
      record_ttl: 200
```